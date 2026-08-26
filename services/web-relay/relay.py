#!/usr/bin/env python3
import asyncio, json, logging, sys, re
from collections import defaultdict
import websockets

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
log = logging.getLogger("atlas-relay")

ATLS_MAGIC = b"ATLS"
ATLS_FULL_HEADER = 36
INPUT_MAGIC = b"INPT"
INPUT_HEADER_SIZE = 13
devices = defaultdict(dict)

async def relay_host_to_ws(device_id, ws, reader):
    buf = bytearray()
    try:
        while True:
            chunk = await reader.read(8192)
            if not chunk:
                log.info(f"[RELAY] Host closed connection for {device_id}")
                break
            buf.extend(chunk)
            while len(buf) >= ATLS_FULL_HEADER:
                if buf[:4] == ATLS_MAGIC:
                    payload_len = int.from_bytes(buf[8:12], "big")
                    total = ATLS_FULL_HEADER + payload_len
                    if len(buf) >= total:
                        w = int.from_bytes(buf[12:16], "big")
                        h = int.from_bytes(buf[16:20], "big")
                        codec = int.from_bytes(buf[20:22], "big")
                        log.info(f"[RELAY] Frame: w={w} h={h} codec={codec}")
                        await ws.send(bytes(buf[:total]))
                        del buf[:total]
                        continue
                if buf[:4] == INPUT_MAGIC and len(buf) >= INPUT_HEADER_SIZE:
                    plen = int.from_bytes(buf[6:8], "little")
                    total = INPUT_HEADER_SIZE + plen
                    if len(buf) >= total:
                        del buf[:total]
                        continue
                if b"\n" in buf:
                    pos = buf.index(b"\n")
                    line = buf[:pos].decode("utf-8", errors="replace")
                    del buf[:pos + 1]
                    log.info(f"[RELAY] Host text: {line}")
                    m = re.match(r"^PAIR_OK:(.+)$", line)
                    if m:
                        host_device_id = m.group(1).strip()
                        paired_msg = json.dumps({"type": "pair", "deviceId": host_device_id, "pairCode": ""})
                        await ws.send(paired_msg)
                        log.info(f"[RELAY] Sent pair: {paired_msg}")
                        continue
                    if line == "PAIR_FAIL":
                        await ws.send(json.dumps({"type": "pair_fail", "reason": "Invalid pair code"}))
                        await ws.close(1008, "Pair code rejected")
                        return
                    await ws.send(line)
                    continue
                break
    except Exception as e:
        log.error(f"[RELAY] Host->WS error for {device_id}: {e}")

async def relay_ws_to_host(device_id, ws):
    try:
        async for msg in ws:
            if isinstance(msg, bytes):
                msg = msg.decode("utf-8", errors="replace")
            text = msg.strip()
            if not text:
                continue
            try:
                data = json.loads(text)
            except json.JSONDecodeError:
                log.warning(f"[RELAY] Invalid JSON: {text[:100]}")
                continue
            cmd = None
            t = data.get("type", "")
            if t == "mouse_move":
                x = data.get("x", 0.0)
                y = data.get("y", 0.0)
                cmd = f"MOUSE_MOVE:{x:.6f}:{y:.6f}"
            elif t == "mouse_click":
                btn = "2" if data.get("button") == "right" else "1"
                pressed = data.get("pressed", True)
                st = "DOWN" if pressed else "UP"
                cmd = f"MOUSE_CLICK:{btn}:{st}"
            elif t == "wheel":
                delta = data.get("delta", 0)
                cmd = f"SCROLL:{delta}"
            elif t == "key":
                code = data.get("code", "")
                pressed = data.get("pressed", True)
                act = "DOWN" if pressed else "UP"
                cmd = f"KEY_{act}:{code}"
            elif t == "double_click":
                cmd = "DOUBLE_CLICK"
            elif t == "pair_accept":
                code = data.get("code", "")
                cmd = f"PAIR_ACCEPT:{code}"
            else:
                log.warning(f"[RELAY] Unknown type: {t}")
                continue
            if cmd and device_id in devices:
                tw = devices[device_id].get("tcp_writer")
                if tw:
                    try:
                        tw.write((cmd + "\n").encode())
                        await tw.drain()
                        log.info(f"[RELAY] -> Host: {cmd}")
                    except Exception as e:
                        log.error(f"[RELAY] Send to host failed: {e}")
    except websockets.exceptions.ConnectionClosed:
        log.info(f"[RELAY] WS closed for {device_id}")
    except Exception as e:
        log.error(f"[RELAY] WS->Host error for {device_id}: {e}")

async def handle_client(ws, path=None):
    query = ""
    req_path = ws.request.path if hasattr(ws, "request") and ws.request else path
    if req_path and "?" in req_path:
        query = req_path.split("?")[1]
    params = {}
    if query:
        for part in query.split("&"):
            if "=" in part:
                k, v = part.split("=", 1)
                params[k] = v
    device_id = params.get("device", "local")
    pair_code = params.get("code", "")
    host_port = int(params.get("host-port", 9090))
    log.info(f"[WS] New connection device={device_id} code={pair_code[:3]}***")
    try:
        tcp_reader, tcp_writer = await asyncio.wait_for(
            asyncio.open_connection("127.0.0.1", host_port), timeout=5.0
        )
    except Exception as e:
        log.error(f"[WS] Failed to connect to host: {e}")
        await ws.close(1013, f"Cannot connect to host:{host_port}")
        return
    pair_msg = f"PAIR:{device_id}:{pair_code}" + "\n"
    tcp_writer.write(pair_msg.encode())
    await tcp_writer.drain()
    log.info(f"[WS] Sent PAIR to host: {pair_msg.strip()}")
    devices[device_id]["ws"] = ws
    devices[device_id]["tcp_reader"] = tcp_reader
    devices[device_id]["tcp_writer"] = tcp_writer
    host_to_ws = asyncio.create_task(relay_host_to_ws(device_id, ws, tcp_reader))
    ws_to_host = asyncio.create_task(relay_ws_to_host(device_id, ws))
    try:
        await asyncio.gather(host_to_ws, ws_to_host)
    except Exception as e:
        log.warning(f"[WS] Connection ended device={device_id}: {e}")
    finally:
        host_to_ws.cancel()
        ws_to_host.cancel()
        tcp_writer.close()
        devices.pop(device_id, None)
        log.info(f"[WS] Disconnected device={device_id}")

async def main():
    host_port = int(sys.argv[1]) if len(sys.argv) > 1 else 9090
    ws_port = int(sys.argv[2]) if len(sys.argv) > 2 else 8080
    log.info(f"AtlasWebRelay v0.4.0 host={host_port} ws={ws_port}")
    async with websockets.serve(handle_client, "0.0.0.0", ws_port, origins=None):
        log.info(f"Relay listening on ws://0.0.0.0:{ws_port}")
        await asyncio.Future()

if __name__ == "__main__":
    asyncio.run(main())
