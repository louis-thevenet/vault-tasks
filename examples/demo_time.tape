# Output ./demo_time.gif
# Output ./demo_time.mp4

Set Shell fish
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
Type "j"
Type "e"
Sleep 1s
Backspace
Backspace
Backspace
Backspace
Backspace
Type "0:5"
Enter 1
Space 2
Sleep 2
Screenshot demo_time_view.png
Type "aaa" # so that the screenshot is actually produced

# Stop vault-tasks
Ctrl+c
