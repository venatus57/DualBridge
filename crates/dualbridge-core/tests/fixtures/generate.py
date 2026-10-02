#!/usr/bin/env python3
"""Regenerates the synthesized input report fixtures in this directory.

These are *synthesized* from the documented report layouts, not captured from
hardware. Captured reports can be added next to them (same .hex format: one
report per file, hex bytes separated by whitespace, '#' starts a comment).

Run: python3 generate.py
"""
import struct
import zlib
from pathlib import Path

HERE = Path(__file__).parent


def touch(active, tid, x, y):
    return bytes([(0 if active else 0x80) | tid, x & 0xFF, ((x >> 8) & 0x0F) | ((y & 0x0F) << 4), y >> 4])


def ds4_data():
    d = bytearray(63)
    d[0:4] = bytes([0x10, 0x20, 0xE0, 0xF0])  # LX LY RX RY
    d[4] = 0x20 | 0x02  # cross + D-pad right
    d[5] = 0x01 | 0x20  # L1 + Options
    d[6] = 0x01 | (5 << 2)  # PS + counter 5
    d[7], d[8] = 0x40, 0xFF  # L2, R2
    d[9:11] = struct.pack("<H", 0x1234)  # timestamp
    d[12:18] = struct.pack("<hhh", 100, -200, 300)  # gyro
    d[18:24] = struct.pack("<hhh", -1, 8192, -8192)  # accel
    d[29] = 0x10 | 5  # cable + level 5
    d[32] = 1  # one touch packet
    d[34:38] = touch(True, 3, 100, 200)
    d[38:42] = touch(False, 2, 0, 0)
    return d


def dualsense_data(edge=False):
    d = bytearray(63)
    d[0:4] = bytes([0x00, 0xFF, 0x80, 0x7F])
    d[4], d[5] = 0x11, 0x22  # L2, R2
    d[6] = 0x42  # counter
    d[7] = 0x80 | 0x10 | 0x07  # triangle + square + D-pad up-left
    d[8] = 0x02 | 0x10 | 0x80  # R1 + Create + R3
    d[9] = 0x02 | 0x04 | (0xF0 if edge else 0)  # touchpad click + mute (+ Edge Fn/paddles)
    d[15:21] = struct.pack("<hhh", -5, 6, -7)
    d[21:27] = struct.pack("<hhh", 10, -20, 8000)
    d[27:31] = struct.pack("<I", 0xDEADBEEF)
    d[32:36] = touch(True, 1, 1919, 1079)
    d[36:40] = touch(True, 2, 0, 0)
    d[41], d[42] = 0x21, 0x12  # right / left trigger status
    d[52] = 0x10 | 7  # charging, level 7
    d[53] = 0x01 | 0x08  # headphones + USB data
    return d


def write(name, report, comment):
    lines = [f"# {comment}", f"# {len(report)} bytes"]
    for i in range(0, len(report), 16):
        lines.append(" ".join(f"{b:02X}" for b in report[i:i + 16]))
    (HERE / name).write_text("\n".join(lines) + "\n")


def bt_sign(report):
    crc = zlib.crc32(bytes([0xA1]) + bytes(report[:-4])) & 0xFFFFFFFF
    report[-4:] = struct.pack("<I", crc)
    return report


write("ds4_usb.hex", bytes([0x01]) + ds4_data(), "DualShock 4, USB report 0x01 (synthesized)")

bt = bytearray(78)
bt[0:3] = bytes([0x11, 0xC0, 0x00])
bt[3:3 + 63] = ds4_data()[:63]
write("ds4_bt.hex", bt_sign(bt), "DualShock 4, Bluetooth report 0x11 (synthesized, valid CRC)")

write("dualsense_usb.hex", bytes([0x01]) + dualsense_data(), "DualSense, USB report 0x01 (synthesized)")
write("dualsense_edge_usb.hex", bytes([0x01]) + dualsense_data(edge=True), "DualSense Edge, USB report 0x01 (synthesized)")

bt = bytearray(78)
bt[0:2] = bytes([0x31, 0x10])
bt[2:2 + 63] = dualsense_data()[:63]
bt[65:74] = bytes(9)
write("dualsense_bt.hex", bt_sign(bt), "DualSense, Bluetooth report 0x31 (synthesized, valid CRC)")

simple = bytes([0x01]) + ds4_data()[:9]
write("bt_simple.hex", simple, "Bluetooth simple report 0x01, before full mode (synthesized)")
