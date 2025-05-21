# Instructions

VAN: Please initialize the memory bank using your deep understanding of the codebase. Store the memory bank in ./.cursor/memory.

Enter PLAN mode: please carefully review the code and plan to add more functionalities, unit tests and great documentation.

Switch to PLAN mode: thoroughly examine the code and strategize to enhance it with additional functionalities, comprehensive unit tests, and excellent documentation. Ensure that the memory bank is updated accordingly.

Enter IMPLEMENT mode

since tonic-mock is a testing crate, do you think any of test support code under @common  is useful to be put to the crate itself?

I've move bench out of tests. And please keep criterion version to 0.6. Please update your memory and do not change the relevant code in @Cargo.toml . Now implement next task.

Please add unit test for this functionality and once finished implement next task

The @bidirectional_test_example.rs hang with info. And doc test also hang (should be bidirectional related) please help fix

This doesn't fix the issue - a good library should be easy to use correctly and hard to misuse it. Please see if you need to improve @lib.rs to make it hard to hang.

This is current output, it didn't hang but crashed

Please focus on these tasks. I've added tokio-test. Please help to make it easy to mock a gRPC request (given a grpc full name, T: Message for request, it should generate a http2 request), gRPC response (same).

Now please update @tasks.md and @README.md and review all docs in code and see if need update.

I've changed respond_with/respond_when/register_handler/reset to async fn. Please fix doc tests and update @README.md accordingly
