# Witness-based reputation system simulation 
Most development was done first in these two forks, and then moved to this repo:
- https://github.com/kianfay/streams-examples/tree/FYP
- https://github.com/kianfay/streams-examples/tree/FYP-part-2

## How to run
Before running the simulation, you need to clone and run one-click-tangle on your local device. More information can be found in the readme of the repo. The repo is found here: https://github.com/iotaledger/one-click-tangle

Use the examples in the /examples directory to quickly run the simulation or evaluation. An example of how to run an example is included in main.rs. Using a similar approach, the other examples can be run. When one or more examples has been selected, run the following from a terminal in the repository.
```bash
cargo run --release
```

### Example output:
```
Selecting participants to be participants and witnesses:
-- Participant 0 is finding witnesses:
---- Found witnesses at indices: [0, 1]
-- Participant 1 is finding witnesses:
---- Found witnesses at indices: [0, 1]

Assigning tranascting nodes as (dis)honest according to their reliability:
-- Trying participant 1. Rand=0.23152721
---- Participant 1 set to honest

Assigning witnesses as (dis)honest according to their reliability:
---- Participant 0 set to dishonest
---- Participant 1 set to honest

Witnesses decide on the outcome:
-- Witnesses 0 responds dishonestly about participant 0 (true)
-- Witnesses 0 responds dishonestly about participant 1 (false)
-- Witnesses 1 responds honestly about participant 0
-- Witnesses 1 responds honestly about participant 1

Verdicts:
-- Participant 1 (TN1)
---- tns [1.0, 1.0] wns [0.0, 1.0]
-- Participant 2 (TN2)
---- tns [1.0, 0.0] wns [1.0, 0.0]
-- Witness node 1 (WN1)
---- tns [1.0, 0.0] wns [1.0, 0.0]
-- Witness node 1 (WN2)
---- tns [1.0, 1.0] wns [0.0, 1.0]
```

### Commentary:
This outcome used the tsg_organization function, so each participant predicts the outcome and then
generates the verdicts. This would obviously not be ideal for the nodes involved in the interaction,
who have hypothetially have complete knowledge about the outcome. Nonetheless, we examine their verdicts
in the context of the organizations they are a part of. TN1 and WN2 are from one, and TN2 and WN1 are
from another. We see that WN1 is responding dishonestly (randomly in this case) and instead of stating
that TN2's outcome was true, they wrongly state it was false. The participants which are part of this
organization will give additional weighting to WN1's statement, and in this case predict the outcome to
be [true,false]. The fact that WN1 is damaging TN2's reputation does not get taken into account by WN1.
Therefore, WN1 and even TN2 predict that TN2 was dishonest, and they both predict that WN1 was honest,
as it is WN1's results that they both predicted. 
