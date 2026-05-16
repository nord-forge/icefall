# FE-262: Scaling and instances UI

**Phase:** 31
**Priority:** High
**Size:** L
**Dependencies:** BE-261

## Description

Dashboard components for managing app scaling and viewing instance health.

## Create

- `dashboard/src/islands/app-detail/InstancesTab/InstancesTab.tsx` — new tab showing all running instances in a table: server name, status badge, container ID, port, uptime, health, actions (stop/restart)
- `dashboard/src/islands/app-detail/SettingsTab/components/ScalingCard.tsx` — desired instances slider/input (1-10), LB policy dropdown, sticky sessions toggle, health check path input
- `dashboard/src/islands/app-detail/OverviewTab/components/InstancesSummary.tsx` — compact view: "3/3 instances healthy" with mini server badges

## Modify

- `dashboard/src/islands/app-detail/` — add "Instances" tab (between Deploys and Logs)
- `SettingsTab.tsx` — add ScalingCard before ServerPlacementCard
- `OverviewTab.tsx` — add InstancesSummary after status section

## Acceptance Criteria

- Given an app with 3 instances, when the instances tab is opened, then all 3 are shown with server, status, and health
- Given the scaling card, when the user changes desired instances to 5 and saves, then 2 new instances begin deploying
- Given an unhealthy instance, when viewed in the instances tab, then it shows a warning badge and a "Restart" button

## Out of Scope

Instance logs (use existing log viewer filtered by server), instance metrics charts
