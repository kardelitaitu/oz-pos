#!/usr/bin/env python3
"""
Script to refactor Rust files by moving inline test modules to sibling *_tests.rs files.

Usage:
    python split_rs_test.py --file <path_to_rs_file> [--line <line_number>]

    --file: Path to the .rs file to refactor
    --line: Optional 1-indexed line number where the #[cfg(test)] mod tests { starts.
            If not provided, the script will search for the pattern.

Example:
    python split_rs_test.py --file crates/oz-core/src/store_profile.rs
"""

import sys
import os
import re

def find_test_module_start(lines):
    """
    Find the start line of the test module.
    Returns (start_line, brace_line) where:
      start_line: the line index of the #[cfg(test)] line
      brace_line: the line index of the line containing the opening brace '{' of the module
    If not found, returns (None, None)
    """
    # First, try to find a line that has both #[cfg(test)] and mod tests { on the same line
    for idx, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith('#[cfg(test)]') and 'mod tests {' in stripped:
            # We found it on one line
            # Now, we need to find the position of the '{' in this line
            # But for brace counting, we'll consider this line as having the opening brace
            return idx, idx

    # If not found on one line, try to find #[cfg(test)] on a line and then mod tests { on a subsequent line
    for idx, line in enumerate(lines):
        stripped = line.strip()
        if stripped == '#[cfg(test)]':
            # Look ahead for the mod tests { line, skipping empty lines and other attributes
            j = idx + 1
            while j < len(lines):
                stripped_j = lines[j].strip()
                if stripped_j == '':
                    j += 1
                    continue
                if stripped_j.startswith('#['):
                    # Skip other attributes like #[allow(deprecated)]
                    j += 1
                    continue
                if stripped_j.startswith('mod tests {'):
                    # Found: the #[cfg(test)] line is at idx, and the brace line is at j
                    return idx, j
                break  # Neither attribute, empty, nor mod tests — stop looking
    return None, None

def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)

    # Parse arguments
    file_path = None
    line_number = None  # 1-indexed, if provided
    i = 1
    while i < len(sys.argv):
        if sys.argv[i] == "--file":
            i += 1
            file_path = sys.argv[i]
        elif sys.argv[i] == "--line":
            i += 1
            line_number = int(sys.argv[i])  # 1-indexed
        i += 1

    if file_path is None:
        print("Error: --file is required")
        sys.exit(1)

    # Read the file
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except FileNotFoundError:
        print(f"Error: File not found: {file_path}")
        sys.exit(1)

    # If line_number is provided, we assume it points to the #[cfg(test)] line
    if line_number is not None:
        start_line = line_number - 1
        # Validate that the line exists
        if start_line < 0 or start_line >= len(lines):
            print(f"Error: Line number {line_number} is out of range (1-{len(lines)})")
            sys.exit(1)
        # We'll use this line as the start of the test module, and we assume the opening brace is on the same line
        # But we should verify that the line contains '{'? Not necessary for now.
        brace_line = start_line
    else:
        # Find the test module start
        start_line, brace_line = find_test_module_start(lines)
        if start_line is None:
            print("Error: Could not find #[cfg(test)] followed by mod tests { in the file")
            sys.exit(1)

    # Now, we know that the test module starts at start_line (the #[cfg(test)] line)
    # and the opening brace is on brace_line.
    # We start brace counting from brace_line.
    brace_count = 0
    found_end = False
    end_line = None
    for idx in range(brace_line, len(lines)):
        line = lines[idx]
        # Count the braces in this line
        for char in line:
            if char == '{':
                brace_count += 1
            elif char == '}':
                brace_count -= 1
                if brace_count == 0:
                    # We found the matching closing brace
                    end_line = idx
                    found_end = True
                    break
        if found_end:
            break

    if not found_end or end_line is None:
        print("Error: Could not find matching closing brace for the test module")
        sys.exit(1)

    # Now, the test module is from start_line to end_line (inclusive)
    # We want to extract the body: from the line after the brace_line to the line before end_line
    # But note: the brace_line has the opening brace, so the body starts after that brace.
    # We need to extract the content after the '{' on brace_line and before the '}' on end_line.

    # We'll extract the body as:
    #   - The part of brace_line after the first '{'
    #   - The lines from brace_line+1 to end_line-1 (inclusive)
    #   - The part of end_line before the first '}'

    # Extract the part after '{' on brace_line
    brace_line_content = lines[brace_line]
    brace_pos = brace_line_content.find('{')
    if brace_pos == -1:
        print(f"Error: Could not find '{{' in line {brace_line+1}")
        sys.exit(1)
    brace_line_after = brace_line_content[brace_pos+1:]

    # Extract the part before '}' on end_line
    end_line_content = lines[end_line]
    brace_pos_end = end_line_content.find('}')
    if brace_pos_end == -1:
        print(f"Error: Could not find '}}' in line {end_line+1}")
        sys.exit(1)
    end_line_before = end_line_content[:brace_pos_end]

    # The body lines are:
    #   [brace_line_after] (if not empty) +
    #   lines[brace_line+1 : end_line] +
    #   [end_line_before] (if not empty)
    body_parts = []
    if brace_line_after.strip() != '' or brace_line_after == '':  # we want to keep empty strings? Actually, we want to keep it as a line only if it's not empty? But note: it might be whitespace.
        # We'll keep it as is, and let the dedenting handle whitespace.
        body_parts.append(brace_line_after)
    # Add the middle lines
    body_parts.extend(lines[brace_line+1:end_line])
    if end_line_before.strip() != '' or end_line_before == '':
        body_parts.append(end_line_before)

    # Now, we have the body as a list of strings (each string is a line, but note: the first and last might be partial lines)
    # We want to dedent the body. However, the first and last parts might not have newline at the end.
    # We'll convert to a list of lines by splitting by newline? But note: the parts we have are already without the newline at the end of the line (because we read with readlines and then took substrings).
    # Actually, the elements in body_parts are strings that do not include the newline (because we took substrings of the line without the newline? Let's see:
    #   lines[brace_line] includes the newline at the end? Yes, because we read with readlines.
    #   Then, brace_line_after = brace_line_content[brace_pos+1:] includes the newline if it was after the '{'.
    #   Similarly, end_line_before does not include the newline? Actually, end_line_content includes the newline, and we took a substring, so it includes the newline if the '}' was before the newline.
    #
    # To avoid confusion, let's work with the entire body as a single string and then split into lines for dedenting.
    #
    # Alternatively, we can treat each element in body_parts as a line (they might have newline at the end) and then dedent by lines.
    #
    # Let's do: convert body_parts to a list of lines by ensuring each element ends with a newline? Not necessary.
    #
    # Instead, let's create a list of lines for the body by:
    #   - If brace_line_after is not empty, add it as a line (it might have a newline at the end if the original line had content after the '{' and then a newline)
    #   - Add the middle lines (which already have newlines)
    #   - If end_line_before is not empty, add it as a line (it might have a newline at the end if the original line had content before the '}' and then a newline)
    #
    # But note: the middle lines (lines[brace_line+1:end_line]) are exactly as read, so they include the newline.
    #
    # We'll do:
    body_lines = []
    if brace_line_after != '':
        body_lines.append(brace_line_after)
    body_lines.extend(lines[brace_line+1:end_line])
    if end_line_before != '':
        body_lines.append(end_line_before)

    # Now, dedent the body_lines
    if body_lines:
        # Convert to string for dedenting? Or work line by line.
        # We'll compute the minimum indentation of non-empty lines.
        min_indent = None
        for line in body_lines:
            stripped = line.lstrip()
            if stripped == '':  # empty line
                continue
            # Count leading spaces
            indent = len(line) - len(stripped)
            if min_indent is None or indent < min_indent:
                min_indent = indent
        if min_indent is not None and min_indent > 0:
            # Remove min_indent spaces from the beginning of each line
            new_body_lines = []
            for line in body_lines:
                if line.strip() == '':
                    new_body_lines.append(line)
                else:
                    new_body_lines.append(line[min_indent:])
            body_lines = new_body_lines

    # Now, the test file content is the body_lines (joined by nothing, because each line already has its newline? 
    # But note: the first and last parts might not have a newline if they were extracted from the middle of a line.
    # We want to ensure that the test file ends with a newline? Not necessary, but we want each line to be separated by newline.
    #
    # Actually, the body_lines list contains strings that may or may not end with a newline.
    # We want to join them with newline? But that would double the newline if they already have it.
    #
    # Let's instead: we know that the middle lines (lines[brace_line+1:end_line]) have newlines.
    # The first and last parts might not.
    #
    # To make it simple, we'll convert the body to a single string and then split into lines by newline, then dedent, then join by newline.
    #
    # But note: we already extracted the body as a list of string fragments. Let's combine them into a single string and then split by newline.
    #
    body_string = ''.join(body_lines)
    # Now, split the body_string into lines
    body_lines_for_dedent = body_string.splitlines(keepends=False)
    # But note: we lost the newline information. We'll dedent and then add newlines when writing.
    #
    # Alternatively, we can work with the string and dedent by lines without splitting? We'll do the splitlines method.
    #
    # Let's do:
    #   lines_list = body_string.splitlines(keepends=False)
    #   then dedent the lines_list
    #   then the test content is '\n'.join(dented_lines_list) + ('\n' if we want to ensure newline at end?)
    #
    # But note: the original body_string might not end with a newline. We'll not add one unless the original did.
    #
    # However, for simplicity, we'll ensure the test file ends with a newline.
    #
    # Let's do the dedenting on the list of lines (without newlines).
    #
    if body_lines_for_dedent:
        min_indent = None
        for line in body_lines_for_dedent:
            stripped = line.lstrip()
            if stripped == '':
                continue
            indent = len(line) - len(stripped)
            if min_indent is None or indent < min_indent:
                min_indent = indent
        if min_indent is not None and min_indent > 0:
            new_body_lines = []
            for line in body_lines_for_dedent:
                if line.strip() == '':
                    new_body_lines.append(line)
                else:
                    new_body_lines.append(line[min_indent:])
            body_lines_for_dedent = new_body_lines
        test_content = '\n'.join(body_lines_for_dedent) + '\n'
    else:
        test_content = ''

    # Determine the test file name
    dir_name = os.path.dirname(file_path)
    base_name = os.path.basename(file_path)
    # Remove the .rs extension
    if base_name.endswith('.rs'):
        stem = base_name[:-3]
    else:
        stem = base_name
    test_file_name = stem + "_tests.rs"
    test_file_path = os.path.join(dir_name, test_file_name)

    # Write the test file
    try:
        with open(test_file_path, 'w', encoding='utf-8') as f:
            f.write(test_content)
    except IOError as e:
        print(f"Error: Could not write test file {test_file_path}: {e}")
        sys.exit(1)

    # Now, replace the test module in the production file
    # We want to replace from start_line to end_line (inclusive) with a single line
    new_test_line = f'#[cfg(test)] #[path = "{test_file_name}"] mod tests;\n'
    new_lines = lines[:start_line] + [new_test_line] + lines[end_line+1:]

    # Write the production file back
    try:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
    except IOError as e:
        print(f"Error: Could not write production file {file_path}: {e}")
        sys.exit(1)

    print(f"Successfully refactored {file_path}")
    print(f"Created test file: {test_file_path}")

if __name__ == '__main__':
    main()