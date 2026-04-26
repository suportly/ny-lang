import sys
import subprocess

out = subprocess.run(["python3", "grep.py"], capture_output=True, text=True)
print(out.stdout)
