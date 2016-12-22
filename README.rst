indexing
========

“Sound unchecked indexing” in Rust using “generativity” (branding by unique
lifetime parameter).

Extremely experimental, but somewhat promising & exciting.

Main focus is on index ranges, not just single indices.

|build_status|_ |crates|_

.. |build_status| image:: https://travis-ci.org/bluss/indexing.svg?branch=master
.. _build_status: https://travis-ci.org/bluss/indexing

.. |crates| image:: http://meritbadge.herokuapp.com/indexing
.. _crates: https://crates.io/crates/indexing

**Crate Features:**

- ``use_std`` Enabled by default, disable to be ``no_std``-compatible.

References
----------

+ Inspired by Gankro’s exposition of `sound unchecked indexing`__.

__ https://www.reddit.com/r/rust/comments/3oo0oe/sound_unchecked_indexing_with_lifetimebased_value/

Also now described in: `You can't spell trust without Rust <https://raw.githubusercontent.com/Gankro/thesis/master/thesis.pdf>`_. Chapter *6.3 hacking generativity onto rust*. Gankro's master's thesis.


Recent Changes
--------------

- 0.3.0

  - Tweak implementation traits a bit, ``PointerRange``, ``Provable``,
    ``ContainerRef``, make them ``unsafe`` where needed.
  - Add ``Container::range_of``

- 0.2.0

  - Docs are better
  - Refactor most of the crate, prepare for other backends than slices
  - Expose ``PIndex, PRange, PSlice`` which are the pointer-based equivalents
    of safe trusted indices and ranges. Some algos are better when using
    a raw pointer representation (for example: lower bound). Since we don't
    have HKT, traitifying all of this is not so pleasant and is not yet complete.
  - New feature: can combine trusted indices with push/insert on Vec.

- 0.1.2

  - Add ``binary_search_by`` and ``lower_bound`` to algorithms. Algorithms
    don't require ``T: Debug`` anymore.

- 0.1.1

  - Point documentation to docs.rs

- 0.1.0

  - Add some docs and tests
  - Fix Range::join_cover_both to use ProofAdd

- 0.1.0-alpha3

  - Add IndexingError and use it for all Results.

- 0.1.0-alpha2

  - Add ProofAdd and use it in Range::join, Range::join_cover
  - Make Index<'id>, Range<'id> Send + Sync

- 0.1.0-alpha1

  - First release


License
-------

Dual-licensed to be compatible with the Rust project.

Licensed under the Apache License, Version 2.0
http://www.apache.org/licenses/LICENSE-2.0 or the MIT license
http://opensource.org/licenses/MIT, at your
option. This file may not be copied, modified, or distributed
except according to those terms.
