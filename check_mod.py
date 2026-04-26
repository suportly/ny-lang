import sys

def check_resolver():
    with open('src/semantic/resolver.rs', 'r') as f:
        content = f.read()
    
    print("find_similar_struct:", "fn find_similar_struct" in content)
    print("test_resolve_unknown_variable_with_suggestion:", "test_resolve_unknown_variable_with_suggestion" in content)
    print("unknown variable did you mean:", "did you mean" in content)

check_resolver()
