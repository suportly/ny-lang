import subprocess

out = subprocess.run(["python3", "run7.py"], capture_output=True, text=True)
print(out.stdout)

out2 = subprocess.run(["python3", "check_mod.py"], capture_output=True, text=True)
print(out2.stdout)
