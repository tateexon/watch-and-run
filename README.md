# Watch and run command

just seeing how hard it is to create a simple watch utility that will run commands upon seeing file changes in the provided directory. It uses your git projects .gitignore file to exclude files to watch.

how to use:

```
#<path-to-executable> <path-to-directory-to-watch> "<command-to-run>"
watch-and-run $(pwd) "make build"
```