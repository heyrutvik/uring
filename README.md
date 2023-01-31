# uring

The `io_uring` library for Rust.

Note: Not for production _at all_ (it doesn't even provide full features). It was mainly built while I was learning about **io_uring** and **unsafe capabilities of Rust**. See [this](https://www.linkedin.com/posts/heyrutvik_a-month-left-in-my-self-funded-sabbatical-activity-7015350751216459776-PBt7/) for context.

Primary goal was to support `read` operation and write `cat` utility using it (check examples). If I get enough time and motivation, I would like to make it a bit more usable and build async runtime around it.
