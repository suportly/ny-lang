import subprocess

out = subprocess.run(["python3", "run5.py"], capture_output=True, text=True)
print(out.stdout)
