import re

with open("server.rs") as f:
    content = f.read()

lines = content.split("\n")

newlines = []
for line in lines:
    if "&perms::build(\"" in line:
        # Convert &perms::build(a1, a2) to &"<a1>.<a2>".into()
        converted_line = re.sub(r'&perms::build\(("\w+"), ("\w+")\)', r'&\1.\2.into()', line)
        newlines.append(converted_line)
    else:
        newlines.append(line)

print("\n".join(newlines))