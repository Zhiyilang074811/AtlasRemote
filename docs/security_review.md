# Security Review - Atlas Remote

## 1. Private Key Storage

### Where are private keys stored?
- Windows Host: DPAPI encrypted, stored in `%APPDATA%\AtlasRemote\keys\`
- Android Client: Android Keystore (hardware-backed)
- Keys NEVER leave the device

### Backup strategy
- Export public key + encrypted backup (requires device password)
- No cloud storage of private keys

## 2. Server Trust Model

### What the relay server CAN see:
- Device IDs (for routing)
- Network metadata (IP, port)
- Signaling messages (device pairing info)

### What the relay server CANNOT see:
- Screen content (encrypted end-to-end)
- Keyboard/mouse events (encrypted end-to-end)
- Device private keys
- Session keys

## 3. Session Management

### Session invalidation
- Explicit disconnect message
- Timeout after 30 minutes of inactivity
- Device pair revocation
- Key rotation every 24 hours

### Revocation
- Remove device from trusted list
- Invalidates all future sessions with that device
- Existing sessions continue until timeout

## 4. Replay Attack Prevention

### Nonce-based replay protection
- Each QUIC stream uses unique nonce
- Nonce counter incremented per message
- Server rejects duplicate nonces

### Timestamp validation
- Messages older than 5 seconds are rejected
- NTP-synced clocks required

## 5. MITM Protection

### How MITM is prevented
- Device pairing uses out-of-band verification (QR code scan)
- Public key fingerprints compared visually
- Certificate pinning for relay server
- Mutual authentication (both sides verify each other)

### Attack scenarios prevented
- Passive eavesdropping: End-to-end encryption
- Active MITM: Device pairing verification
- Replay attacks: Nonce + timestamp
- Relay server compromise: No plaintext data on server

## 6. Sensitive Data in Logs

### PROHIBITED in logs:
- Private keys
- Session keys
- Keyboard input
- Mouse coordinates
- Screen content
- Passwords
- Authentication tokens

### ALLOWED in logs:
- Connection state changes
- Error messages (without sensitive data)
- Performance metrics
- Device IDs (non-sensitive)

## 7. Network Security

### Protocols used
- QUIC with TLS 1.3
- AES-256-GCM for data encryption
- Ed25519 for device authentication
- X25519 for key exchange

### Firewall requirements
- UDP 443 for QUIC
- Optional TCP fallback on 443



