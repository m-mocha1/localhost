#!/usr/bin/env python3
import sys
import os

body = sys.stdin.read()

print("Content-Type: text/plain\n")
print("Hello from Python!")
print("PATH_INFO:", os.environ.get("PATH_INFO"))
print("Body:", body)