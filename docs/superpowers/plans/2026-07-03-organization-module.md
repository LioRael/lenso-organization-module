# Organization Module Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a first-party `organization` linked module for reusable SaaS organization, membership, invitation, and configurable role access.

**Architecture:** Create a sibling `lenso-organization-module` repository with a single Rust linked module crate. The module depends on `auth`, owns its own schema, exposes `organization::module::linked_module()`, and keeps role permissions local instead of injecting them into actor scopes.

**Tech Stack:** Rust 2024, SQLx/Postgres, Axum, Lenso linked module manifests, `platform-module` admin data/actions, `platform-testing`, GitHub Actions.

---

This checked-in plan records the execution target for the organization module
v1. The implementation scope is: schema, repository, public helpers, routes,
manifest, admin data/actions, CI, README, and a `lenso-cli` linked install
descriptor.

## Completed Tasks

- [x] Scaffold new sibling repo and `lenso-module-organization` crate.
- [x] Add organization schema migrations for organizations, roles, memberships, and invitations.
- [x] Add repository methods for organizations, roles, memberships, and invitations.
- [x] Add manifest, capabilities, HTTP metadata, admin surface metadata, and linked module entrypoint.
- [x] Add public Rust helpers for create/list/permission/accept flows.
- [x] Add HTTP routes for create/list/member list/invite/accept flows.
- [x] Add admin data and admin actions for invitations and membership changes.
- [x] Add integration tests for repository helpers, admin actions, route flows, and permission denial.
- [x] Add README, security note, Cargo lockfile, and GitHub Actions CI.
- [x] Add `organization` builtin linked descriptor to `lenso-cli`.
