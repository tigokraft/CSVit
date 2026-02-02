# Generate a 2.5GB CSV File
# Rows: ~40M
# Format: id, name, value, description

import random
import string

def generate_random_string(length):
    return ''.join(random.choices(string.ascii_letters, k=length))

def main():
    filename = "large_test.csv"
    target_size_bytes = 100 * 1024 * 1024 # 100 MB for quick test
    
    with open(filename, "w") as f:
        f.write("id,name,value,description\n")
        current_size = 0
        i = 0
        while current_size < target_size_bytes:
            id = str(i)
            name = generate_random_string(10)
            value = str(random.randint(0, 10000))
            desc = generate_random_string(50)
            line = f"{id},{name},{value},{desc}\n"
            f.write(line)
            current_size += len(line)
            i += 1
            if i % 100000 == 0:
                print(f"Written {i} rows, {current_size / 1024 / 1024:.2f} MB")

    print("Done!")

if __name__ == "__main__":
    main()
