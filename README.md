# indexing
“Sound unchecked indexing” in Rust using branding

Extremely experimental, but somewhat promising & exciting.

Main focus is on index ranges, not just single indices.

```
// This program demonstrates sound unchecked indexing
// by having slices generate valid indices, and "signing"
// them with an invariant lifetime. These indices cannot be used on another
// slice, nor can they be stored until the array is no longer valid
// (consider adapting this to Vec, and then trying to use indices after a push).
//
// This represents a design "one step removed" from iterators, providing greater
// control to the consumer of the API. Instead of getting references to elements
// we get indices, from which we can get references or hypothetically perform
// any other "index-related" operation (slicing?). Normally, these operations
// would need to be checked at runtime to avoid indexing out of bounds, but
// because the array knows it personally minted the indices, it can trust them.
// This hypothetically enables greater composition. Using this technique
// one could also do "only once" checked indexing (let idx = arr.validate(idx)).
//
// The major drawback of this design is that it requires a closure to
// create an environment that the signatures are bound to, complicating
// any logic that flows between the two (e.g. moving values in/out and try!).
// In principle, the compiler could be "taught" this trick to eliminate the
// need for the closure, as far as I know. Although how one would communicate
// that they're trying to do this to the compiler is another question.
// It also relies on wrapping the structure of interest to provide a constrained
// API (again, consider applying this to Vec -- need to prevent `push` and `pop`
// being called). This is the same principle behind Entry and Iterator.
//
// It also produces terrible compile errors (random lifetime failures),
// because we're hacking novel semantics on top of the borrowchecker which
// it doesn't understand.
//
// This technique was first pioneered by gereeter to enable safely constructing
// search paths in BTreeMap. See Haskell's ST Monad for a related design.
//
// The example isn't maximally generic or fleshed out because I got bored trying
// to express the bounds necessary to handle &[T] and &mut [T] appropriately.
```
