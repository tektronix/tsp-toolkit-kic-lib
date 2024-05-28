# PR Checklist:

- [x] If refactoring: Were regression tests (unit, integration, system) created before changes occurred to ensure no functionality was lost?
    - N/A
- [x] Have the changes been verified against a representative set of DUTs?
    - N/A
- [x] Have appropriate documentation and in-line comments been added
    - Documentation is pretty sparse.
        - **Minor**: Add explanation of thread spawning
        - **Minor**: Add better explanation of match arms
        - **Minor**: For TODO item on L89, add a when clause and potentially a
          Jira Ticket reference AND be more specific about what name.
        - **Minor**: Document reason for sleep on L98
        - **Minor**: Fix comment on L90
        - **Minor**: Add comment below L91 to indicate the new thread is taking
          ownership of `read_into` and `write_out`
        - **Minor**: Add comment stating the purpose of the `mpsc::channel`s on L84-L85
        - **Minor**: L129 Justify sleep
        - **Minor**: L111 Add label to `loop` and associated `break`s for clarity
- [x] Have examples been added to documentation comments (testable in Rust)?
    - **Minor**: Doc Comment should be added with example of usage
- [x] Update changelog in dependent projects? (find a way to add to CI?)
    - WIP
- [x] Is there a simpler way to accomplish this?
    - **Minor**: There should be another effort to look at using Rust async instead, as
      that would remove the need for this function entirely.
- [x] Is the proper API exposed?
    - Yes, trait implementations are always public for public types
- [x] Are there any side-effects from the code?
    - Yes, but it is recorded in the signature, an as convention: The converted type will be consumed.
    - **Minor** This should be added to the documentation comment
- [x] Could the new functionality be shared with other portions of the code?
    - No
- [x] Have applicable comments been updated?
    - N/A All new code
- [x] Are included dependencies necessary? Can features be turned off?
    - N/A No dependencies
- [x] Have new dependencies been properly reviewed for health (CVEs, update frequency, testing, etc.)?
    - N/A no new dependencies
- [x] Has any schedule-induced or otherwise known technical debt been sufficiently noted via in-line comment in the code AND in the current issue tracking system?
    - Yes, TODO comments. This should be moved to Jira.
- [x] Have applicable usage notes been updated?
    - Add doc comment noted above
- [x] What corner-cases are missing?
    - None, we hand Reads and Writes, which are all that we need.
- [x] How would this be exhaustively tested?
    - This is very testable via dependency injection: Use any type that implements `std::io::Read` and `std::io::Write`.
- [x] Is any portion of the code "doing too much"?
    - **Minor**: It is possible it should be broken down into a couple other functions
      and/or should have another type that owns the thread loop
- Additional
    - **Minor**: L114 This comment should accurately describe WHY not use a newline:
    ```rust
    // Do NOT add a newline here. Users of Read or Write should add it themselves
    // to keep this a raw R/W interface`
    ```

## Consolidated Feedback (14 Actions)

- **Major**: Unit Testing!
- **Minor**: There should be another effort to look at using Rust async instead, as that would remove the need for this function entirely.
- **Minor**: It is possible it should be broken down into a couple other functions and/or should have another type that owns the thread loop
- **Minor**: L078 Add details of about consumption of converted type
- **Minor**: L078 Doc Comment should be added with example of usage
- **Minor**: L084-L085 Add comment stating the purpose of the `mpsc::channel`s
- **Minor**: L089 add a when clause and potentially a Jira Ticket reference AND be more specific about what name.
- **Minor**: L090 Comment unclear
- **Minor**: L091 Add explanation of thread spawning
- **Minor**: L093-L094 Add comment below L91 to indicate the new thread is taking ownership of `read_into` and `write_out`
- **Minor**: L098 Document reason for sleep on L98
- **Minor**: L111 Add label to `loop` and associated `break`s for clarity
- **Minor**: L112-L134 Add better explanation of match arms
- **Minor**: L114 This comment should accurately describe WHY not use a newline:
    ```rust
    // Do NOT add a newline here. Users of Read or Write should add it themselves
    // to keep this a raw R/W interface`
    ```
- **Minor**: L129 Justify sleep
