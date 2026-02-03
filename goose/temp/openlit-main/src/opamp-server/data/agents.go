package data

import (
	"crypto/sha256"
	"io"
	"log"
	"os"
	"path/filepath"
	"sync"

	"github.com/google/uuid"
	"github.com/open-telemetry/opamp-go/protobufs"
	"github.com/open-telemetry/opamp-go/protobufshelpers"
	"github.com/open-telemetry/opamp-go/server/types"
	"gopkg.in/yaml.v3"

	"opamp-server/constants"
)

type Agents struct {
	mux         sync.RWMutex
	agentsById  map[InstanceId]*Agent
	connections map[types.Connection]map[InstanceId]bool
}

var logger = log.New(log.Default().Writer(), "[AGENTS] ", log.Default().Flags()|log.Lmsgprefix|log.Lmicroseconds)

// RemoveConnection removes the connection all Agent instances associated with the
// connection.
func (agents *Agents) RemoveConnection(conn types.Connection) {
	agents.mux.Lock()
	defer agents.mux.Unlock()

	for instanceId := range agents.connections[conn] {
		delete(agents.agentsById, instanceId)
	}
	delete(agents.connections, conn)
}

func (agents *Agents) SetCustomConfigForAgent(
	agentId InstanceId,
	config *protobufs.AgentConfigMap,
	notifyNextStatusUpdate chan<- struct{},
) {
	agent := agents.FindAgent(agentId)
	if agent != nil {
		agent.SetCustomConfig(config, notifyNextStatusUpdate)
	}
}

func isEqualAgentDescr(d1, d2 *protobufs.AgentDescription) bool {
	if d1 == d2 {
		return true
	}
	if d1 == nil || d2 == nil {
		return false
	}
	return isEqualAttrs(d1.IdentifyingAttributes, d2.IdentifyingAttributes) &&
		isEqualAttrs(d1.NonIdentifyingAttributes, d2.NonIdentifyingAttributes)
}

func isEqualAttrs(attrs1, attrs2 []*protobufs.KeyValue) bool {
	if len(attrs1) != len(attrs2) {
		return false
	}
	for i, a1 := range attrs1 {
		a2 := attrs2[i]
		if !protobufshelpers.IsEqualKeyValue(a1, a2) {
			return false
		}
	}
	return true
}

func (agents *Agents) FindAgent(agentId InstanceId) *Agent {
	agents.mux.RLock()
	defer agents.mux.RUnlock()
	return agents.agentsById[agentId]
}

func (agents *Agents) FindOrCreateAgent(agentId InstanceId, conn types.Connection) *Agent {
	agents.mux.Lock()
	defer agents.mux.Unlock()

	// Ensure the Agent is in the agentsById map.
	agent := agents.agentsById[agentId]
	if agent == nil {
		agent = NewAgent(agentId, conn)
		agents.agentsById[agentId] = agent

		// Load persisted configuration if available
		agents.loadPersistedConfig(agent)

		// Ensure the Agent's instance id is associated with the connection.
		if agents.connections[conn] == nil {
			agents.connections[conn] = map[InstanceId]bool{}
		}
		agents.connections[conn][agentId] = true
	}

	return agent
}

// loadPersistedConfig loads a persisted configuration for an agent if it exists
func (agents *Agents) loadPersistedConfig(agent *Agent) {
	agentIDStr := uuid.UUID(agent.InstanceId).String()
	configPath := filepath.Join(constants.ConfigDirectory, agentIDStr+".yaml")

	// Check if persisted config exists
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		return
	}

	// Read the persisted configuration
	configData, err := os.ReadFile(configPath)
	if err != nil {
		logger.Printf("Failed to read persisted config for agent %s: %v", agentIDStr, err)
		return
	}

	// Validate that the persisted config is valid YAML
	var yamlTest interface{}
	if err := yaml.Unmarshal(configData, &yamlTest); err != nil {
		logger.Printf("Persisted config for agent %s is invalid YAML, skipping: %v", agentIDStr, err)
		// Delete the corrupted config file
		if removeErr := os.Remove(configPath); removeErr != nil {
			logger.Printf("Failed to remove corrupted config file %s: %v", configPath, removeErr)
		} else {
			logger.Printf("Removed corrupted config file: %s", configPath)
		}
		return
	}

	// Additional validation: ensure it's a YAML map/object, not a scalar value
	if _, isMap := yamlTest.(map[string]interface{}); !isMap {
		logger.Printf("Persisted config for agent %s must be a YAML object, not a scalar value. Skipping.", agentIDStr)
		// Delete the invalid config file
		if removeErr := os.Remove(configPath); removeErr != nil {
			logger.Printf("Failed to remove invalid config file %s: %v", configPath, removeErr)
		} else {
			logger.Printf("Removed invalid config file: %s", configPath)
		}
		return
	}

	// Set the custom config on the agent
	agent.CustomInstanceConfig = string(configData)
	agent.calcRemoteConfig()

	logger.Printf("Loaded persisted configuration for agent %s from %s", agentIDStr, configPath)
}

func (agents *Agents) GetAgentReadonlyClone(agentId InstanceId) *Agent {
	agent := agents.FindAgent(agentId)
	if agent == nil {
		return nil
	}

	// Return a clone to allow safe access after returning.
	return agent.CloneReadonly()
}

func (agents *Agents) GetAllAgentsReadonlyClone() map[InstanceId]*Agent {
	agents.mux.RLock()

	// Clone the map first
	m := map[InstanceId]*Agent{}
	for id, agent := range agents.agentsById {
		m[id] = agent
	}
	agents.mux.RUnlock()

	// Clone agents in the map
	for id, agent := range m {
		// Return a clone to allow safe access after returning.
		m[id] = agent.CloneReadonly()
	}
	return m
}

func (a *Agents) OfferAgentConnectionSettings(
	id InstanceId,
	offers *protobufs.ConnectionSettingsOffers,
) {
	if len(offers.Hash) == 0 {
		offers.Hash = toHash(offers)
	}

	a.mux.Lock()
	defer a.mux.Unlock()

	agent, ok := a.agentsById[id]
	if ok {
		agent.OfferConnectionSettings(offers)
		logger.Printf("Client connection settings offers sent to %s (hash=%x)\n", id, offers.Hash)
	} else {
		logger.Printf("Agent %s not found\n", id)
	}
}

var AllAgents = Agents{
	agentsById:  map[InstanceId]*Agent{},
	connections: map[types.Connection]map[InstanceId]bool{},
}

// toHash computes a sha256 hash from fields within ConnectionSettingsOffers
func toHash(c *protobufs.ConnectionSettingsOffers) []byte {
	hasher := sha256.New()
	if c.Opamp != nil {
		hashEndpoint(hasher, c.Opamp)
	}
	if c.OwnMetrics != nil {
		hashEndpoint(hasher, c.OwnMetrics)
	}
	if c.OwnTraces != nil {
		hashEndpoint(hasher, c.OwnTraces)
	}
	if c.OwnLogs != nil {
		hashEndpoint(hasher, c.OwnLogs)
	}
	for name, endpoint := range c.OtherConnections {
		hasher.Write([]byte(name))
		hashEndpoint(hasher, endpoint)
	}

	return hasher.Sum(nil)
}

// endpoint is an incomplete interface to assist with turning an offered connection into a hash.
type endpoint interface {
	GetDestinationEndpoint() string
	GetHeaders() *protobufs.Headers
	GetCertificate() *protobufs.TLSCertificate
	GetTls() *protobufs.TLSConnectionSettings
}

// hashEndpoint writes some shared attributes of the passed enpoint to the passed writer.
func hashEndpoint(w io.Writer, e endpoint) {
	_, err := w.Write([]byte(e.GetDestinationEndpoint()))
	if err != nil {
		panic(err)
	}
	if headers := e.GetHeaders(); headers != nil {
		for _, header := range headers.Headers {
			_, err := w.Write([]byte(header.Key + header.Value))
			if err != nil {
				panic(err)
			}
		}
	}
	if cert := e.GetCertificate(); cert != nil {
		_, err := w.Write(cert.Cert)
		if err != nil {
			panic(err)
		}
		_, err = w.Write(cert.PrivateKey)
		if err != nil {
			panic(err)
		}
		_, err = w.Write(cert.CaCert)
		if err != nil {
			panic(err)
		}
	}
	if tlsSettings := e.GetTls(); tlsSettings != nil {
		_, err := w.Write([]byte(tlsSettings.CaPemContents))
		if err != nil {
			panic(err)
		}
	}
}
