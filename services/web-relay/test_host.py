import asyncio, struct, time, sys, random, logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s [%(levelname)s] %(message)s')
log = logging.getLogger('atlas-host')
ATLS_MAGIC = b'ATLS'
ATLS_FULL_HEADER = 36
INPUT_MAGIC = b'INPT'
INPUT_HEADER = 13
def crc32_atls(data):
    crc = 0xFFFFFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = (crc >> 1) ^ 0xEDB88320 if crc & 1 else crc >> 1
    return crc ^ 0xFFFFFFFF
def make_frame(width, height, payload, ts):
    codec = 2
    plen = len(payload)
    h = ATLS_MAGIC
    h += struct.pack('>H', 1)
    h += struct.pack('>H', 1)
    h += struct.pack('>I', plen)
    h += struct.pack('>I', width)
    h += struct.pack('>I', height)
    h += struct.pack('>I', ts & 0xFFFFFFFF)
    h += struct.pack('>H', codec)
    crc = crc32_atls(h + payload)
    return h + struct.pack('>H', codec) + struct.pack('>I', crc) + payload
async def handle(reader, writer):
    addr = writer.get_extra_info('peername')
    did = f'test-{random.randint(1000,9999)}'
    code = f'{random.randint(100000,999999)}'
    log.info(f'[HOST] Client from {addr}')
    log.info(f'[HOST] Device: {did}')
    log.info(f'[HOST] Pair Code: {code}')
    try:
        data = await asyncio.wait_for(reader.read(1024), timeout=10)
        text = data.decode('utf-8','replace').strip()
        log.info(f'[HOST] Received: {text}')
        if text.startswith('PAIR:'):
            parts = text.split(':')
            if len(parts) >= 3:
                log.info(f'[HOST] Paired: {parts[1]}')
                writer.write((f'PAIR_OK:{did}' + chr(10)).encode())
                await writer.drain()
            else:
                log.warning(f'[HOST] Malformed pair')
                return
    except Exception as e:
        log.error(f'[HOST] Pair error: {e}')
        return
    fc = 0
    async def send_loop():
        nonlocal fc
        while True:
            ts = int(time.time()*1000) & 0xFFFFFFFF
            payload = bytes([(fc*7)%256, ((fc*13)+128)%256, ((fc*17)+64)%256, 255] * 100)
            pkt = make_frame(1920, 1080, payload, ts)
            try:
                writer.write(pkt)
                await writer.drain()
                fc += 1
                if fc % 10 == 0:
                    log.info(f'[HOST] Sent {fc} frames')
            except Exception as e:
                log.error(f'[HOST] Send error: {e}')
                break
            await asyncio.sleep(1/30)
    st = asyncio.create_task(send_loop())
    log.info('[HOST] send_loop started')
    buf = b''
    try:
        while True:
            chunk = await asyncio.wait_for(reader.read(4096), timeout=30.0)
            if not chunk: break
            buf += chunk
            while b'\n' in buf:
                p = buf.index(b'\n')
                line = buf[:p].decode('utf-8','replace').strip()
                buf = buf[p+1:]
                if line:
                    log.info(f'[HOST] Input: {line}')
    except asyncio.TimeoutError:
        log.info('[HOST] Timeout')
    except Exception as e:
        log.error(f'[HOST] Error: {e}')
    finally:
        st.cancel()
        await writer.close()
        log.info(f'[HOST] Done, {fc} frames')
async def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 9090
    srv = await asyncio.start_server(handle, '0.0.0.0', port)
    log.info(f'AtlasRemote Test Host v0.2.0')
    log.info(f'Listening on 0.0.0.0:{port}')
    async with srv:
        await srv.serve_forever()
if __name__ == '__main__':
    asyncio.run(main())
