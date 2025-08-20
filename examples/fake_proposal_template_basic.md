# ACME Proposal – Data Platform

**Client:** City of Metropolis  
**Submission Date:** 2025-08-19

## Executive Summary

This proposal outlines a scalable, secure, cloud-native data platform to meet the City’s requirements. We address ingestion, processing, storage, and user access.

## Requirements Overview

- **R1:** Ingest streaming sensor data with sub-minute latency.
- **R2:** Provide role-based dashboards and search.
- **R3:** Satisfy CJIS data handling requirements.

## Technical Approach

### Architecture Overview

![System Architecture](./architecture.png "System Architecture")

### Key Components

1. **Ingestion:** Kafka with schema registry.
2. **Processing:** Flink jobs with autoscaling.
3. **Storage:** Object storage + columnar warehouse.
4. **Interfaces:** REST + GraphQL + OData.

## Code Sample

```python
def greet(name: str) -> str:
    return f"Hello, {name}!"
print(greet("Metropolis"))
```