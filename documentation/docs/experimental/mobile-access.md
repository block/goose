---
title: Mobile Access via Secure Tunneling
sidebar_position: 3
sidebar_label: Mobile Access
description: Enable remote access to goose from mobile devices using secure tunneling.
---

import { PanelLeft } from 'lucide-react';

Mobile access lets you connect to goose remotely from an iOS mobile device using secure tunneling.

:::warning Experimental Feature
Mobile access is a preview feature in active development. Behavior and configuration may change in future releases.
:::

## How Mobile Access Works

Mobile access connects your iOS device to goose Desktop through a secure tunnel. After you install and configure the **Goose AI** app, you can access goose from anywhere.

**Key details:**
- Uses [Lapstone](https://github.com/michaelneale/lapstone-tunnel), a public HTTPS tunnel service provided by Mic Neale
- Easy setup using a QR code with a unique secret key to secure the connection
- Your tunnel URL remains the same across sessions, so you only need to configure your mobile app once
- The connection requires your computer to be awake with goose Desktop running
- Automatically reconnects if interrupted and restarts when you launch goose Desktop

## Setup

### Install the App
1. Install the **Goose AI** app on your iOS mobile device from the [App Store](https://apps.apple.com/app/goose-ai/id6752889295)

### Start the Tunnel
1. Open goose Desktop
2. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
3. Click `Settings` in the sidebar
4. Click `App`
5. Scroll down to the `Remote Access` section and click `Start Tunnel`

Once the tunnel starts, you'll see a `Remote Access Connection` QR code for configuring the app.

:::info 
Click `Stop Tunnel` at any time to close the connection.
:::

### Connect the App
1. Open the **Goose AI** app on your iOS mobile device
2. Scan the `Remote Access Connection` QR code displayed in goose Desktop
3. The app will automatically configure the connection

You can now access goose Desktop from your mobile device.

## What You Can Do

The mobile app gives you full access to goose:
- Start new conversations or continue existing sessions
- Use all your goose extensions and configurations
- Work from anywhere while your computer handles the processing