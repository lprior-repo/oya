import os
import re

attr_file = "#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]\n"
attr_block = "#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]\n"

def process_tests_dir():
    tests_dir = "tests"
    for filename in os.listdir(tests_dir):
        if filename.endswith(".rs"):
            filepath = os.path.join(tests_dir, filename)
            with open(filepath, "r") as f:
                content = f.read()
            
            if attr_file in content:
                continue
            
            # Find the position to insert. After //! comments.
            lines = content.splitlines(keepends=True)
            insert_idx = 0
            for i, line in enumerate(lines):
                if line.startswith("//!"):
                    insert_idx = i + 1
                elif line.strip() == "":
                    continue
                else:
                    break
            
            lines.insert(insert_idx, attr_file)
            with open(filepath, "w") as f:
                f.writelines(lines)
            print(f"Updated {filepath}")

def process_mod_tests():
    # Find all .rs files
    for root, dirs, files in os.walk("src"):
        for filename in files:
            if filename.endswith(".rs"):
                filepath = os.path.join(root, filename)
                with open(filepath, "r") as f:
                    content = f.read()
                
                # Check for inline mod tests {
                if "mod tests {" in content and attr_block not in content:
                    content = content.replace("mod tests {", attr_block + "mod tests {")
                    with open(filepath, "w") as f:
                        f.write(content)
                    print(f"Updated {filepath} (inline)")
                
                # Check for out-of-line mod tests;
                if "mod tests;" in content:
                    # Look for tests.rs or tests/mod.rs in the same directory
                    # Actually, if it's mod.rs, it looks for tests.rs in the same dir.
                    # If it's something.rs, it looks for something/tests.rs.
                    parent_dir = root
                    if filename == "mod.rs":
                        potential_test_file = os.path.join(parent_dir, "tests.rs")
                    else:
                        potential_test_file = os.path.join(parent_dir, filename[:-3], "tests.rs")
                    
                    if os.path.exists(potential_test_file):
                        with open(potential_test_file, "r") as f:
                            test_content = f.read()
                        
                        if attr_file not in test_content:
                            test_lines = test_content.splitlines(keepends=True)
                            test_lines.insert(0, attr_file)
                            with open(potential_test_file, "w") as f:
                                f.writelines(test_lines)
                            print(f"Updated {potential_test_file} (out-of-line)")

if __name__ == "__main__":
    process_tests_dir()
    process_mod_tests()
