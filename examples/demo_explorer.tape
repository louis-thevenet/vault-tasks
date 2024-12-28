Output ./demo_explorer.gif
Output ./demo_explorer.mp4
# Output ./demo_explorer.webm
# Output ./frames/

Set Shell zsh
Set Theme "Builtin Solarized Light"
Set TypingSpeed 80ms
Set FontSize 22
Set Width 1800
Set Height 1000

Hide
Type "vault-tasks -c ../.config/ -v ../test-vault explorer"
Enter
Show

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
Sleep 600ms
Type ":wq"
Enter

Sleep 2s

Ctrl+c
