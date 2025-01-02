Output ./demo_calendar.gif
# Output ./demo_calendar.mp4

Set Shell zsh
Set Theme "Builtin Solarized Light"
Set TypingSpeed 80ms
Set FontSize 22
Set Width 1800
Set Height 1000

Hide
Type "vault-tasks -c ../.config/ -v ../test-vault calendar"
Enter
Show
# Calendar tab
# 
Sleep 2.5s

Right
Sleep 2s
Right@0.5s 4
Sleep 2s

Right@0.5s 3
Sleep 1s
Right@1s 2
Sleep 1s

Shift+k
Sleep 1s
Up@0.5s 3

# Stop vault-tasks
Ctrl+c

