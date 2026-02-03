package server

import (
	"context"
	"crypto/tls"
	"fmt"
	"log"
	"net/http"
	"os"
	"strings"

	"github.com/oklog/ulid/v2"
	"go.opentelemetry.io/contrib/instrumentation/net/http/otelhttp"

	"opamp-server/certman"
	"opamp-server/config"
	"opamp-server/constants"
	"opamp-server/data"

	"github.com/open-telemetry/opamp-go/protobufs"
	"github.com/open-telemetry/opamp-go/server"
	"github.com/open-telemetry/opamp-go/server/types"
)

type Server struct {
	opampSrv server.OpAMPServer
	httpSrv  *http.Server
	agents   *data.Agents
	logger   *OpAMPLogger
	config   *config.ServerConfig
}

func NewServer(agents *data.Agents) *Server {
	// Load configuration
	cfg := config.NewConfig()

	logger := &OpAMPLogger{
		log.New(
			log.Default().Writer(),
			"[OPAMP] ",
			log.Default().Flags()|log.Lmsgprefix|log.Lmicroseconds,
		),
	}

	// Update certificate paths if custom directory is specified
	if cfg.TLS.CertificatesDirectory != constants.CertificatesDirectory {
		constants.UpdateCertificatePaths(cfg.TLS.CertificatesDirectory)
	}

	srv := &Server{
		agents: agents,
		logger: logger,
		config: cfg,
	}

	srv.opampSrv = server.New(logger)

	mux := http.NewServeMux()
	srv.setupRoutes(mux)
	srv.httpSrv = &http.Server{
		Addr:    cfg.GetAPIEndpoint(),
		Handler: mux,
	}

	logger.Printf("Starting API server on %s", cfg.GetAPIEndpoint())
	go srv.httpSrv.ListenAndServe()

	return srv
}

func (srv *Server) Start() {
	srv.logger.Printf("Starting OpAMP server in %s environment", srv.config.TLS.Environment)

	// Validate configuration
	if err := srv.config.Validate(); err != nil {
		if srv.config.IsProduction() {
			srv.logger.Errorf(context.Background(), "Configuration validation failed in production: %v", err)
			os.Exit(1)
		} else {
			srv.logger.Printf("WARNING: Configuration validation failed (continuing in %s mode): %v", srv.config.TLS.Environment, err)
		}
	}

	settings := server.StartSettings{
		Settings: server.Settings{
			Callbacks: types.Callbacks{
				OnConnecting: func(request *http.Request) types.ConnectionResponse {
					return types.ConnectionResponse{
						Accept: true,
						ConnectionCallbacks: types.ConnectionCallbacks{
							OnMessage:         srv.onMessage,
							OnConnectionClose: srv.onDisconnect,
						},
					}
				},
			},
		},
		ListenEndpoint: srv.config.GetListenEndpoint(),
		HTTPMiddleware: otelhttp.NewMiddleware("/v1/opamp"),
	}

	// Create TLS configuration with environment-aware settings
	tlsConfig, err := srv.createTLSConfig()
	if err != nil {
		if srv.config.IsProduction() {
			srv.logger.Errorf(context.Background(), "Failed to create TLS config in production: %v", err)
			os.Exit(1)
		} else {
			srv.logger.Printf("WARNING: Could not load TLS config, working without TLS in %s mode: %v", srv.config.TLS.Environment, err)
		}
	} else {
		srv.logger.Printf("TLS configuration loaded successfully")
	}

	settings.TLSConfig = tlsConfig

	srv.logger.Printf("Starting OpAMP server on %s", srv.config.GetListenEndpoint())
	if err := srv.opampSrv.Start(settings); err != nil {
		srv.logger.Errorf(context.Background(), "OpAMP server start failed: %v", err.Error())
		os.Exit(1)
	}
}

// createTLSConfig creates TLS configuration based on environment settings
func (srv *Server) createTLSConfig() (*tls.Config, error) {
	// Convert string TLS versions to constants
	minVersion, err := srv.parseTLSVersion(srv.config.TLS.MinTLSVersion)
	if err != nil {
		return nil, fmt.Errorf("invalid min TLS version: %v", err)
	}

	maxVersion, err := srv.parseTLSVersion(srv.config.TLS.MaxTLSVersion)
	if err != nil {
		return nil, fmt.Errorf("invalid max TLS version: %v", err)
	}

	options := certman.TLSConfigOptions{
		InsecureSkipVerify: srv.config.TLS.InsecureSkipVerify,
		RequireClientCert:  srv.config.TLS.RequireClientCert,
		MinTLSVersion:      minVersion,
		MaxTLSVersion:      maxVersion,
	}

	return certman.CreateServerTLSConfigWithOptions(
		constants.CaCertPath,
		constants.ServerCertPath,
		constants.ServerCertKeyPath,
		options,
	)
}

// parseTLSVersion converts string TLS version to tls constant
func (srv *Server) parseTLSVersion(version string) (uint16, error) {
	switch strings.ToLower(version) {
	case "1.0":
		return tls.VersionTLS10, nil
	case "1.1":
		return tls.VersionTLS11, nil
	case "1.2":
		return tls.VersionTLS12, nil
	case "1.3":
		return tls.VersionTLS13, nil
	default:
		return 0, fmt.Errorf("unsupported TLS version: %s", version)
	}
}

func (srv *Server) Stop() {
	srv.opampSrv.Stop(context.Background())
}

func (srv *Server) onDisconnect(conn types.Connection) {
	srv.agents.RemoveConnection(conn)
}

func (srv *Server) onMessage(ctx context.Context, conn types.Connection, msg *protobufs.AgentToServer) *protobufs.ServerToAgent {
	// Start building the response.
	response := &protobufs.ServerToAgent{}

	var instanceId data.InstanceId
	if len(msg.InstanceUid) == 26 {
		// This is an old-style ULID.
		u, err := ulid.Parse(string(msg.InstanceUid))
		if err != nil {
			srv.logger.Errorf(ctx, "Cannot parse ULID %s: %v", string(msg.InstanceUid), err)
			return response
		}
		instanceId = data.InstanceId(u.Bytes())
	} else if len(msg.InstanceUid) == 16 {
		// This is a 16 byte, new style UID.
		instanceId = data.InstanceId(msg.InstanceUid)
	} else {
		srv.logger.Errorf(ctx, "Invalid length of msg.InstanceUid")
		return response
	}

	agent := srv.agents.FindOrCreateAgent(instanceId, conn)

	// Process the status report and continue building the response.
	agent.UpdateStatus(msg, response)

	if msg.ConnectionSettingsStatus != nil {
		srv.logger.Debugf(ctx, "Connection settings for instance %x %s (err=%s) hash=%x", instanceId, msg.ConnectionSettingsStatus.Status.String(), msg.ConnectionSettingsStatus.ErrorMessage, msg.ConnectionSettingsStatus.LastConnectionSettingsHash)
	}

	// Send the response back to the Agent.
	return response
}
