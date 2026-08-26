import asyncio, websockets, json, sys
sys.stdout.reconfigure(encoding="utf-8")

async def test():
    print("Connecting...", flush=True)
    ws = await websockets.connect("ws://127.0.0.1:8080/?device=file-test&code=000000")
    print("Connected!", flush=True)
    msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
    print("Got:", msg, flush=True)
    data = json.loads(msg)
    print("Device:", data["deviceId"], flush=True)
    await ws.send(json.dumps({"type": "pair_accept", "code": "000000"}))
    print("Sent pair_accept", flush=True)
    for i in range(5):
        try:
            m = await asyncio.wait_for(ws.recv(), timeout=2.0)
            print(f"Frame {i}: {len(m)} bytes", flush=True)
        except asyncio.TimeoutError:
            print(f"Frame {i}: timeout", flush=True)
            break
    print("SUCCESS", flush=True)
    await ws.close()

asyncio.run(test())
