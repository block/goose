# Voice Dictation Feature - PR Summary

## Overview
This PR adds voice dictation functionality to Goose Desktop, allowing users to input messages using their microphone with support for both OpenAI Whisper and ElevenLabs speech-to-text services.

## Key Features

### 1. Voice Input UI
- **Microphone button** in chat input area (next to send button)
- **Recording indicator** with duration and file size monitoring
- **Real-time waveform visualization** during recording
- **Visual feedback** for recording/transcribing states

### 2. Dual Provider Support
- **OpenAI Whisper**: Uses existing OpenAI API key, no additional configuration needed
- **ElevenLabs Speech-to-Text**: Alternative provider with advanced features
- **Smart provider switching**: Automatically available based on configured API keys

### 3. Settings & Configuration
- New **Voice Dictation** section in Settings
- Toggle to enable/disable the feature
- Provider selection dropdown
- ElevenLabs API key configuration with secure storage
- Provider-specific information and features

### 4. Technical Implementation

#### Backend (Rust)
- New `/audio/transcribe` endpoint for OpenAI Whisper
- New `/audio/transcribe/elevenlabs` endpoint for ElevenLabs
- `/audio/config` endpoint to check provider availability
- 25MB file size limit for both providers
- Support for multiple audio formats (webm, mp3, mp4, m4a, wav)
- Automatic API key migration to secure storage for ElevenLabs

#### Frontend (TypeScript)
- `useWhisper` hook for recording management
- `useDictationSettings` hook for settings persistence
- `WaveformVisualizer` component for audio feedback
- Microphone permission handling
- Real-time size and duration monitoring
- Automatic recording stop at 10 minutes or 25MB

### 5. Security & Privacy
- All API keys stored securely
- Audio data transmitted as base64 over HTTPS
- No audio stored locally after transcription
- Microphone permissions requested only when needed

## File Changes

### New Files
- `crates/goose-server/src/routes/audio.rs` - Audio transcription endpoints
- `ui/desktop/src/hooks/useWhisper.ts` - Recording and transcription logic
- `ui/desktop/src/hooks/useDictationSettings.ts` - Settings management
- `ui/desktop/src/components/settings/dictation/DictationSection.tsx` - Settings UI
- `ui/desktop/src/components/WaveformVisualizer.tsx` - Audio visualization

### Modified Files
- `ui/desktop/src/components/ChatInput.tsx` - Added microphone button
- `ui/desktop/src/components/settings/SettingsView.tsx` - Added dictation section
- `ui/desktop/src/main.ts` - Added microphone permission handling
- `ui/desktop/src/preload.ts` - Exposed permission APIs
- Various server files to register new routes

## Testing
- All Rust tests passing
- TypeScript compilation successful
- ESLint and formatting checks passed
- Manual testing completed with both providers

## Future Enhancements
- Real-time streaming transcription
- Language detection and selection
- Custom vocabulary support
- Local Whisper model support
- Voice activity detection

## Breaking Changes
None - Feature is disabled by default and requires user opt-in.
