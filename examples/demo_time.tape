Output ./demo_time.gif
# Output ./demo_time.mp4

Set Shell zsh
Set Theme "Builtin Solarized Light"
Set TypingSpeed 80ms
Set FontSize 22
Set Width 1800
Set Height 1000

Hide
Type "vault-tasks -c ../.config/ -v ../test-vault time"
Enter
Show

# Time Management tab
Sleep 1s
Space
Sleep 2s

Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms

Sleep 1s
Type "e"
Sleep 1s
Backspace
Backspace
Backspace
Backspace
Backspace
Type "0:20"
Enter
Space
Sleep 6s
Tab
Space
Sleep 6s
Space
Sleep 400ms
Type "p"
Sleep 1
Type "p"
Sleep 300ms
Type "p"
Sleep 1
Type "p"
Sleep 15s

# Stop vault-tasks
Ctrl+c
