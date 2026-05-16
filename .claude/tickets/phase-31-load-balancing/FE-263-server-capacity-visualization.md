# FE-263: Server capacity visualization

**Phase:** 31
**Priority:** Medium
**Size:** S
**Dependencies:** BE-261

## Description

Show instance distribution and remaining capacity on the servers page.

## Create

- `dashboard/src/islands/servers/ServerDetail/components/InstancesSection.tsx` — list of app instances running on this server, grouped by app

## Modify

- Server list page — add "Instances" column showing count per server
- Server detail overview — add InstancesSection showing which apps and how many instances

## Acceptance Criteria

- Given a server running 5 instances across 3 apps, when the server detail is opened, then instances are listed grouped by app
- Given the servers list, when viewed, then each server shows its instance count

## Out of Scope

Capacity planning recommendations, instance placement suggestions
