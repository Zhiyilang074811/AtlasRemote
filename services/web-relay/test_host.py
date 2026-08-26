import asyncio, struct, time, sys, random, logging, os

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
log = logging.getLogger("atlas-host")

ATLS_MAGIC = b"ATLS"
ATLS_FULL_HEADER = 36
INPUT_MAGIC = b"INPT"
INPUT_HEADER = 13

def crc32_atls(data):
    crc = 0xFFFFFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = (crc >> 1) ^ 0xEDB88320 if crc & 1 else crc >> 1
    return crc ^ 0xFFFFFFFF

def make_bgra_frame(width, height, frame_num):
    data = bytearray()
    for y in range(height):
        for x in range(width):
            r = ((x * 3 + frame_num * 20) % 256)
            g = ((y * 2 + frame_num * 15) % 256)
            b = ((x + y + frame_num * 10) % 256)
            a = 255
            data.extend([b, g, r, a])
    return bytes(data)

def make_frame(width, height, payload, ts, codec=0):
    plen = len(payload)
    h = ATLS_MAGIC
    h += struct.pack(">H", 1)
    h += struct.pack(">H", 1)
    h += struct.pack(">I", plen)
    h += struct.pack(">I", width)
    h += struct.pack(">I", height)
    h += struct.pack(">I", ts)
    h += struct.pack(">H", codec)
    crc = crc32_atls(h + payload)
    return h + struct.pack(">H", codec) + struct.pack(">I", crc) + payload

async def handle(reader, writer):
    addr = writer.get_extra_info("peername")
    did = f"test-{random.randint(1000,9999)}"
    code = f"{random.randint(100000,999999)}"
    log.info(f"[HOST] Client from {addr}")
    log.info(f"[HOST] Device: {did}")
    log.info(f"[HOST] Pair Code: {code}")
    with open(r"D:\Android\AtlasRemote\logs\pair_code.txt", "w", encoding="utf-8") as f:
        f.write(code)
    try:
        data = await asyncio.wait_for(reader.read(1024), timeout=10)
        text = data.decode("utf-8","replace").strip()
        log.info(f"[HOST] Received: {text}")
        if text.startswith("PAIR:"):
            parts = text.split(":")
            if len(parts) >= 3:
                log.info(f"[HOST] Paired: {parts[1]}")
                writer.write((f"PAIR_OK:{did}" + "\n").encode())
                await writer.drain()
            else:
                log.warning("[HOST] Malformed pair")
                return
    except Exception as e:
        log.error(f"[HOST] Pair error: {e}")
        return

    WIDTH = 320
    HEIGHT = 240
    frame_num = 0
    async def send_loop():
        nonlocal frame_num
        while True:
            ts = int(time.time()*1000) & 0xFFFFFFFF
            payload = make_bgra_frame(WIDTH, HEIGHT, frame_num)
            pkt = make_frame(WIDTH, HEIGHT, payload, ts, codec=0)
            try:
                writer.write(pkt)
                await writer.drain()
                frame_num += 1
                if frame_num % 30 == 0:
                    log.info(f"[HOST] Sent {frame_num} frames ({WIDTH}x{HEIGHT} BGRA)")
            except Exception as e:
                log.error(f"[HOST] Send error: {e}")
                break
            await asyncio.sleep(1/30)

    st = asyncio.create_task(send_loop())
    log.info(f"[HOST] send_loop started, sending {WIDTH}x{HEIGHT} BGRA at 30fps")
    # Read input with NO timeout - keep connection alive indefinitely
    buf = b""
    try:
        while True:
            chunk = await reader.read(4096)
            if not chunk: break
            buf += chunk
            while b"\n" in buf:
                p = buf.index(b"\n")
                line = buf[:p].decode("utf-8","replace").strip()
                buf = buf[p+1:]
                if line:
                    log.info(f"[HOST] Input: {line}")
    except Exception as e:
        log.error(f"[HOST] Error: {e}")
    finally:
        st.cancel()
        await writer.close()
        log.info(f"[HOST] Done, {frame_num} frames")

async def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 9090
    srv = await asyncio.start_server(handle, "0.0.0.0", port)
    log.info(f"AtlasRemote Test Host v0.5.0 - NO TIMEOUT")
    log.info(f"Listening on 0.0.0.0:{port}")
    async with srv:
        await srv.serve_forever()

if __name__ == "__main__":
    asyncio.run(main())
