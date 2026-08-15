"""Minimal SMTP debug sink for local OTP testing.

Accepts EHLO/MAIL/RCPT/DATA, never advertises STARTTLS or AUTH, prints
each captured message to stdout so the OTP code can be read.
"""
import asyncio

MESSAGES = []


async def handle(reader, writer):
    peer = writer.get_extra_info("peername")
    print(f"[smtp] connection from {peer}")

    def send(line):
        writer.write((line + "\r\n").encode())
        return writer.drain()

    await send("220 sink ESMTP")

    mail_from = None
    rcpt_to = []
    in_data = False
    data_lines = []

    try:
        while True:
            line = (await reader.readline()).decode("utf-8", "replace").rstrip("\r\n")
            # A bare CRLF inside DATA is a blank body line, not a close —
            # only treat an empty read (EOF) or a non-DATA empty line as
            # termination.
            if not line and not in_data:
                break
            if in_data:
                if line == ".":
                    msg = "\n".join(data_lines)
                    MESSAGES.append((mail_from, list(rcpt_to), msg))
                    print(f"[smtp] captured message from={mail_from} to={rcpt_to}")
                    print("----- BEGIN MESSAGE -----")
                    print(msg)
                    print("----- END MESSAGE -----")
                    in_data = False
                    data_lines = []
                    mail_from = None
                    rcpt_to = []
                    await send("250 OK")
                else:
                    data_lines.append(line)
                continue
            upper = line.upper()
            if upper.startswith("EHLO") or upper.startswith("HELO"):
                await send("250-sink")
                await send("250 8BITMIME")
            elif upper.startswith("MAIL FROM"):
                mail_from = line[10:].strip()
                await send("250 OK")
            elif upper.startswith("RCPT TO"):
                rcpt_to.append(line[8:].strip())
                await send("250 OK")
            elif upper == "DATA":
                in_data = True
                await send("354 End data with <CR><LF>.<CR><LF>")
            elif upper.startswith("RSET"):
                mail_from = None
                rcpt_to = []
                await send("250 OK")
            elif upper.startswith("QUIT"):
                await send("221 Bye")
                break
            elif upper.startswith("NOOP"):
                await send("250 OK")
            else:
                await send("250 OK")  # swallow anything else (AUTH etc.)
    except (ConnectionResetError, BrokenPipeError):
        pass
    except Exception as ex:
        import traceback
        print(f"[smtp] handler error: {ex!r}")
        traceback.print_exc()
    finally:
        writer.close()
        try:
            await writer.wait_closed()
        except Exception:
            pass


async def main():
    server = await asyncio.start_server(handle, "127.0.0.1", 1025)
    print("[smtp] sink listening on 127.0.0.1:1025", flush=True)
    async with server:
        await server.serve_forever()


if __name__ == "__main__":
    asyncio.run(main())
