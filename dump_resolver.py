import sys

def main():
    with open('src/semantic/resolver.rs', 'r') as f:
        content = f.read()
    
    # Let's split content into blocks and write them to files or just print
    with open('resolver_full.txt', 'w') as out:
        out.write(content)

if __name__ == "__main__":
    main()
