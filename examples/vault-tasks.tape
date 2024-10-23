# To run:
#
#     vhs < vault-tasks.tape

Output ./vault-tasks.gif
Output ./vault-tasks.mp4
Output ./vault-tasks.webm
# Output ./frames/

Set Shell zsh
Set TypingSpeed 100ms
Set FontSize 22
Set Width 1300
Set Height 650

Type "vault-tasks -v ../test-vault"

Sleep 500ms

Enter

# First entry
Sleep 3s
Down

# Second entry
Sleep 1.5s

# Scroll
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J

Sleep 2s

# Search bar
Type "s"
Backspace 6
Enter

Sleep 2s

Down 4
Enter@1s 3

# Deep into 5th entry
Sleep 2s

# open in editor and mark tasks done
Type "o"
Sleep 1s

Type "4"
Down
Type "7xs- \[ "
Enter
Type ";"
Sleep 1s
Type "rx"
Enter
Type ":wq"
Enter

Sleep 2s

# Filter tab
Shift+L
Sleep 3s
Type "#tobuy 09/29"
Enter

# Scroll
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J

Sleep 4s

# Stop vault-tasks
Ctrl+c
