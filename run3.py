import subprocess
with open("out.txt", "w") as f:
    subprocess.run(["python3", "grep.py"], stdout=f)
