Output ./demo_explorer.gif
# Output ./demo_explorer.mp4
# Output ./demo_explorer.webm
# Output ./frames/

Set Shell fish
Set Theme "Builtin Solarized Light"
Set TypingSpeed 150ms
Set FontSize 22
Set Width 1800
Set Height 1000

Hide
Type "vault-tasks -c ../.config/ -v ../test-vault explorer"
Enter
Show

# First entry is `dir`
Down
# Physics
Sleep 3s

Down
# Vault-tasks
Sleep 3s

# Search bar
Type "s"
Backspace@0.5s 6
Enter
# Now we see every tasks
Down
Down # Back to #3

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

Up@0.5s 2
Enter@0.5s 
Down
Sleep 1.5s

Up@0.5s
Enter
Down
Sleep 1.5s
Down
Sleep 1.5s

# open in editor and mark tasks done
Type "o"
Sleep 1s

Type "/- \[ "
Enter
Type ";"
Sleep 1s
Type "rx"
Sleep 600ms
Type ":wq"
Enter

Sleep 2s

Ctrl+c
