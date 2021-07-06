# Example project to show autotest behaviors

### Reset demo state
This demo has no meaningful state, though a longer running daemon is present to monitor for changes.
Running `./bazelisk shutdown` will ensure its shutdown along with the bazel version in memory.


## Running


`./bazelisk autotest`

Edit something like `Animal.java` or `Cat.java`, all downstream targets should be built and checked when it changes




