# Mesh Governance

**Vision**

Mesh governance is the first IBC mesh application built on the Abstract framework that enables chains and their members to do fully decentralized governance. This application is a cornerstone to improving Cosmos's public goods funding process by allowing individual chains to pool money into an overarching DAO that is tasked with funding **interchain** development.

**Description**

Mesh-IBC allows for every chain to be part of an interchain governance DAO, making proposals that are synced between chains. Each chain can vote on the proposal locally that EVERY chain votes on with their native governance.

The app has an integration with Neutron ICQ to enable querying governance proposals on other chains, including their state (how much voted for vs. against). This query is not available through the native smart-contract as the stargate queries aren't whitelisted. Through this mechanism the governance structure can be made more democratic by taking the amount of votes into account (vs just a YES / NO).

Technologies:

-   Abstract SDK
-   Abstract ICAAS
-   Neutron ICQ
-   IBC

**Features**

**Multi-chain Consensus**

This app syncs data between contracts on multiple different chains over IBC with a system of data change proposals and finalizations.

**Initiation**

Each chain instantiates their own instance of the mesh-gov contract. From there invites are sent to new members. Invites are proposals which means that the DAO needs to agree on approving a new member in the DAO.

**Proposal Execution**

Proposals can contain messages that can be executed on multiple chains. This way proposals can access funds on each participant chain.

**Implemented functionality**

We have a functional e2e test for the application that proposes the addition of a third member in an interchain DAO. Both chains approve the proposal, after which the proposal is executed and the third member is added to the DAO.

You can find the test [here](<https://github.com/CyberHoward/interchain-gov/blob/fb03d6bb40f9c98fa49019ce25d10efd24ee3a02/contracts/interchain-gov/tests/integration.rs#L507>).

**Future work**

Currently voting is a yes/no action. We integrated Neutron's ICQ to enable more nuanced voting but testing infrastructure was too time-consuming to test.

**Sync Steps (Technical)**

-   Local: Propose
-   Remote: Propose Acknowledge
-   Local: Finalize
-   Remote: Finalize Acknowledge
-   Local & Remote: Vote
-   Local: Request vote results through IBC module query, call back with results, containing voting mechanism
-   Local: Sync vote (Propose, Propose Acknowledge, Finalize, Finalize Acknowledge)
-   Local (Neutron): Use ICQ to request governance vote results, called back through ICQ
-   Local: Tally gov votes from ICQ
-   Sync gov votes (Propose, Propose Acknowledge, Finalize, Finalize Acknowledge)
-   Local or Remote: Execute proposal, perform local state change
-   Sync state change: (Propose, Propose Acknowledge, Finalize, Finalize Acknowledge)
