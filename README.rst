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

References
----------

+ Inspired by Gankro’s exposition of `sound unchecked indexing`__.

__ https://www.reddit.com/r/rust/comments/3oo0oe/sound_unchecked_indexing_with_lifetimebased_value/

Also now described in: `You can't spell trust without Rust <https://raw.githubusercontent.com/Gankro/thesis/master/thesis.pdf>`_. Chapter *6.3 hacking generativity onto rust*. Alexis Beingessner's master's thesis.


License
-------

Dual-licensed to be compatible with the Rust project.

Licensed under the Apache License, Version 2.0
http://www.apache.org/licenses/LICENSE-2.0 or the MIT license
http://opensource.org/licenses/MIT, at your
option. This file may not be copied, modified, or distributed
except according to those terms.
