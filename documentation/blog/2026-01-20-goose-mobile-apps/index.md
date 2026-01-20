---
title: "goose mobile apps and agent clients"
description: Consolidating agent apps for ios and android and ACP
authors:
    - mic
---

Recently we consolidated the apps for goose mobile to this repository: https://github.com/block/goose-mobile

In 2025 we did a fairly cutting edge take on whole device automation using android (code name was gosling) which was an on-device agent that would take over your device (mic even used it to do some shopping - which he realized after some things arrived at his door that it had automatically purchased as the result of an email - hence the PoC/experimental label!)

The `goose-ios` client however is more production ready, and in the app store (still early days). We hope to have a port of that to android, which will be strictly a client (and won't take over your device!) to your remote agent. The aim of the client (vs an on device agent) is for you to take your work on the go with you. 

Really great for long running tasks, checking on things, or just shooting off an idea but still keeping things local to your personal agent (where all your stuff is) securely. 

<!-- truncate -->

## Mobile Client Roadmap

### ACP 

As [ACP](https://agentclientprotocol.com/overview/introduction) evolves and matures, it makes sense to have the mobile clients use that to communicate over the tunnel to the goose server (which implements ACP). This has the side benefit of the clients working with any ACP compatible agent. It is reasonable to imagine many clients, and agent servers being in the mix together due to open standards, just like MCP servers (and now skills) can be used between agent implementations, which is a great outcome for everyone.

### Tunnel Technology

For mobile client to work for personal (ie desktop/laptop/PC agents, not really servers) there was a need to allow traffic inbound. Many solutions exist, from hole punching (STUN/TURN etc), Tor, ngrok/cloudflared like services, and VPNs. For general usage for people to try, we have [this solution](https://github.com/michaelneale/lapstone-tunnel) which is what goose uses when you enable a tunnel, using cloudflare with websockets, workers and durable objects to keep things lite and efficient (of course in some enterprise settings you will have access to a VPN so you can adapt the solution to that).

<head>
  <meta property="og:title" content="goose mobile apps" />
  <meta property="og:type" content="article" />
  <meta property="og:url" content="https://block.github.io/goose/blog/2026/01/20/goose-mobile-apps" />
  <meta property="og:description" content="Consolidating agent apps for iOS and Android and ACP" />
  <meta property="twitter:domain" content="block.github.io/goose" />
  <meta name="twitter:title" content="goose mobile apps" />
  <meta name="twitter:description" content="Consolidating agent apps for iOS and Android and ACP" />
</head>
