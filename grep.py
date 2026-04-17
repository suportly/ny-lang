import sys

def main():
    with open('src/semantic/resolver.rs', 'r') as f:
        lines = f.readlines()
    
    for i, line in enumerate(lines):
        if "fn resolve_expr" in line or "fn resolve_stmt" in line or "fn find_similar" in line or "test_" in line:
            print(f"{i}: {line.strip()}")

if __name__ == "__main__":
    main()
