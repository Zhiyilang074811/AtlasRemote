# Atlas Remote - Test Plan

## Phase 1: LAN Control (Computer → Phone)

### Test 1.1: Basic Connection
- [ ] Same WiFi network
- [ ] Computer → Huawei MaiMang 20
- [ ] Measure: latency, FPS, jitter

### Test 1.2: Input Testing
- [ ] Mouse movement
- [ ] Keyboard input
- [ ] Scroll wheel
- [ ] Ctrl+Alt+Del (if supported)

### Test 1.3: Video Quality
- [ ] 1080p @ 60fps
- [ ] 720p @ 30fps (low-end device)
- [ ] Measure: bitrate, CPU usage

## Phase 2: Mobile Network (4G/5G)

### Test 2.1: NAT Traversal
- [ ] Enable VPN on computer
- [ ] Connect from phone 4G/5G
- [ ] Measure: connection time, success rate

### Test 2.2: Relay Fallback
- [ ] Test when direct connection fails
- [ ] Measure: latency increase, quality

### Test 2.3: Reconnection
- [ ] Disconnect and reconnect
- [ ] Measure: recovery time

## Phase 3: Public Network

### Test 3.1: Hong Kong VPN
- [ ] Simulate complex network
- [ ] Measure: stability, packet loss recovery

### Test 3.2: Long Duration
- [ ] Run for 1+ hours
- [ ] Monitor: memory leak, connection stability

---

## Success Criteria

| Metric | Target |
|--------|--------|
| Latency (LAN) | < 50ms |
| Latency (VPN) | < 150ms |
| FPS | 30-60 |
| CPU Usage (Host) | < 10% |
| Packet Loss Recovery | < 2s |



