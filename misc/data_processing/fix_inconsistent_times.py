'''
Attempts to fix inconsistent times (usually times off by <10ms) in the CSV file. 

In the original implementation of the tracker, the timestamp used was
based on when the particular section was saved and not when all similar
sections were saved. 
'''

DELTA = 10
FIXED_CSV_FILE = 'enrollment_fixed.csv'

lines_changed = 0
lined_iterated = 0
with open(FIXED_CSV_FILE, 'w') as fixed_file:
    with open("enrollment.csv", "r") as f:
        init = False 
        prev_time = -1
        for l in f:
            lined_iterated += 1
            if not init:
                fixed_file.write(f'{l}')
                init = True  
                continue 

            line = l.split(',')
            time = int(line[0])
            temp_line = ','.join(line)

            # Initial base case
            if prev_time == -1:
                fixed_file.write(f'{temp_line}')
                prev_time = time 
                continue
                
            # Switched to a different section
            if abs(time - prev_time) > DELTA:
                fixed_file.write(f'{temp_line}')
                prev_time = time 
                continue 

            # Same time
            if time == prev_time:
                fixed_file.write(f'{temp_line}')
                continue 

            # Problematic line
            line[0] = str(prev_time)
            temp_line = ','.join(line)
            fixed_file.write(f'{temp_line}')
            lines_changed += 1

print(f'Fixed {lines_changed} lines (out of {lined_iterated} total lines).')
input()