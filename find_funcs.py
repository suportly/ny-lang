import re

with open('src/semantic/resolver.rs', 'r') as f:
    content = f.read()

# Let's print out the functions we need to modify
def extract_fn(name, content):
    pattern = r"fn " + name + r"[\s\S]*?(?=\n\s*fn |\n\})"
    match = re.search(pattern, content)
    if match:
        print(f"--- {name} ---")
        print(match.group(0)[:500])
        print("...")

extract_fn("resolve_expr", content)
extract_fn("resolve_stmt", content)
