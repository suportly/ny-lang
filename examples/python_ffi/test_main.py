import os

def test_python_ffi_example_files_exist():
    current_dir = os.path.dirname(__file__)
    main_py_path = os.path.join(current_dir, "main.py")
    ml_ops_ny_path = os.path.join(current_dir, "ml_ops.ny")
    
    assert os.path.exists(main_py_path), f"Expected main.py at {main_py_path}"
    assert os.path.exists(ml_ops_ny_path), f"Expected ml_ops.ny at {ml_ops_ny_path}"

def test_python_ffi_content():
    current_dir = os.path.dirname(__file__)
    main_py_path = os.path.join(current_dir, "main.py")
    
    with open(main_py_path, "r") as f:
        content = f.read()
        
    assert "ctypes" in content
    assert "ml_relu" in content
    assert "ml_dot_product" in content
