import subprocess

out = subprocess.run(["python3", "find_funcs.py"], capture_output=True, text=True)
print(out.stdout)
