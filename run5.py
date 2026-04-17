import subprocess

out = subprocess.run(["python3", "mod2.py"], capture_output=True, text=True)
print(out.stdout)
