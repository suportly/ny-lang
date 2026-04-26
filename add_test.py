import sys

with open('tests/compile_run.rs', 'r') as f:
    content = f.read()

if 'fn test_series_methods()' not in content:
    content += '''
#[test]
fn test_series_methods() {
    assert_eq!(compile_and_run("series_methods_test.ny"), 42);
}
'''

    with open('tests/compile_run.rs', 'w') as f:
        f.write(content)
    print("Added test_series_methods")
else:
    print("Already exists")
