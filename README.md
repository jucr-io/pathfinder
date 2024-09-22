<div align="center">

# JCPP Gateway v2

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://github.com/codespaces/new/jucr-io/pathfinder?skip_quickstart=true&machine=standardLinux32gb&repo=514263361&ref=main&devcontainer_path=.devcontainer%2Fdevcontainer.json&location=WestEurope)

Relay between an async message broker like **Apache Kafka** and the [Apollo Router](https://github.com/apollographql/router).

The implementation follows the [Twelve Factors](https://12factor.net).

[Idea](#-idea) •
[Structure](#-structure) •
[APIs](#-apis) •
[Development](#-development) •
[@jucr-io/backend](https://github.com/orgs/jucr-io/teams/backend)

</div>

export ROUTER_ENDPOINT__PORT=3001 &&
export MESSAGE_CONSUMER__KAFKA__BROKERS="kafka-broker:29092" &&
export KV_STORE__REDIS__HOST="redis" &&
export KV_STORE__REDIS__PORT=7559 &&
export KV_STORE__REDIS__PASSWORD="jucr12" &&
export KV_STORE__REDIS__TLS_ENABLED=false &&
