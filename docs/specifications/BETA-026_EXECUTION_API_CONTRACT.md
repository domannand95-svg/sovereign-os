\# BETA-026: Execution API Contract Specification



\*\*Status:\*\* Frozen v1.0  

\*\*Scope:\*\* `sovereign-execution-api` DTO boundary  

\*\*Milestone:\*\* BETA-026



\---



\# 1. Overview



This specification freezes the external contract of the `sovereign-execution-api`.



External consumers must couple to this versioned contract rather than internal Rust implementation details.



The execution API defines the boundary between:



\- external execution requests

\- authorization references

\- kernel invocation

\- execution outcomes

\- audit-controlled diagnostics



\---



\# 2. Architectural Invariants



\## 2.1 Execution Status ≠ Failure Diagnostics



The execution API exposes execution outcomes only.



Detailed kernel diagnostics remain inside controlled audit and ledger boundaries.



The external boundary must not expose internal kernel state.



```

ExecutionStatus != FailureDiagnostics

```



\---



\## 2.2 Authorization ≠ Execution



Authentication and execution failure represent different governance events.



```

AuthenticationFailed != ExecutionFailed

```



Meaning:



| Status | Meaning |

|---|---|

| AuthenticationFailed | Authorization validation failed before execution |

| ExecutionFailed | Execution failed after authorization succeeded |



\---



\# 3. Request Contract



\## GovernedExecutionRequest



Frozen v1 DTO:



```rust

pub struct GovernedExecutionRequest {

&#x20;   pub execution\_id: String,

&#x20;   pub authorization\_receipt\_id: String,

&#x20;   pub operation\_payload: Vec<u8>,

}

```



\---



\## Field Semantics



| Field | Type | Description |

|---|---|---|

| `execution\_id` | `String` | Caller supplied execution correlation identifier |

| `authorization\_receipt\_id` | `String` | Canonical authorization receipt reference |

| `operation\_payload` | `Vec<u8>` | Operation payload bytes |



\---



\## Request Constraints



\### execution\_id



Requirements:



\- Must not be empty

\- Must uniquely identify the execution context

\- Used for request/response correlation



\---



\### authorization\_receipt\_id



Requirements:



\- Must reference a valid authorization receipt

\- Must correspond to the expected authority boundary

\- Must not grant authority by existence alone



\---



\### operation\_payload



Requirements:



\- Passed through the execution boundary

\- Validated before execution

\- Digest and payload integrity checks occur upstream



\---



\# 4. Response Contract



\## GovernedExecutionResponse



Frozen v1 DTO:



```rust

pub struct GovernedExecutionResponse {

&#x20;   pub execution\_id: String,

&#x20;   pub status: ExecutionStatus,

&#x20;   pub report\_reference: Option<String>,

}

```



\---



\# 5. Execution Status Contract



```rust

pub enum ExecutionStatus {

&#x20;   AuthorizedAndExecuted,

&#x20;   AuthenticationFailed,

&#x20;   ExecutionFailed,

}

```



\---



\## Status Definitions



| Status | Definition |

|---|---|

| `AuthorizedAndExecuted` | Authorization succeeded and execution completed successfully |

| `AuthenticationFailed` | Authorization receipt validation failed |

| `ExecutionFailed` | Execution failed after successful authorization |



\---



\## Diagnostic Boundary



`ExecutionFailed` intentionally contains no diagnostic payload.



Incorrect:



```rust

ExecutionFailed("filesystem unavailable".to\_string())

```



Correct:



```rust

ExecutionFailed

```



Detailed diagnostics belong to:



\- audit records

\- execution receipts

\- internal logs



They do not belong in the external execution response.



\---



\# 6. Error Taxonomy



Frozen error categories:



```rust

pub enum ExecutionApiError {

&#x20;   ValidationFailure(String),

&#x20;   AuthorizationFailure(String),

&#x20;   KernelRejection(String),

&#x20;   ExecutionFailure(String),

&#x20;   TransportFailure(String),

}

```



\---



\# 7. Error Precedence Chain



Errors follow deterministic precedence ordering:



```

ValidationFailure

&#x20;       >

AuthorizationFailure

&#x20;       >

KernelRejection

&#x20;       >

ExecutionFailure

&#x20;       >

TransportFailure

```



\---



\## Category Definitions



| Error | Meaning |

|---|---|

| ValidationFailure | Request format, digest, timestamp, or payload validation failure |

| AuthorizationFailure | Receipt or authority validation failure |

| KernelRejection | Kernel explicitly refused the operation |

| ExecutionFailure | Runtime execution failure after authorization |

| TransportFailure | Serialization, encoding, or communication failure |



\---



\# 8. Schema Artifacts



The following machine-readable schemas define this contract:



```

GOVERNED\_EXECUTION\_REQUEST-v1.schema.json



GOVERNED\_EXECUTION\_RESPONSE-v1.schema.json



EXECUTION\_API\_ERROR-v1.schema.json

```



Location:



```

docs/specifications/schemas/

```



\---



\# 9. Versioning



Current version:



```

BETA-026 Execution API Contract v1.0

```



Future changes must follow:



\## Minor Revision



Allowed:



\- additional optional fields

\- non-breaking metadata additions



Example:



```

v1.1

```



\---



\## Major Revision



Required for:



\- breaking DTO changes

\- semantic changes

\- altered error precedence



Example:



```

v2.0

```



\---



\# 10. Implementation Boundary



The execution API guarantees:



```

External Request

&#x20;       |

&#x20;       v

GovernedExecutionRequest

&#x20;       |

&#x20;       v

Authorization Boundary

&#x20;       |

&#x20;       v

Kernel Execution Boundary

&#x20;       |

&#x20;       v

GovernedExecutionResponse

&#x20;       |

&#x20;       v

Audit Retrieval Reference

```



The API exposes controlled execution state.



It does not expose:



\- kernel internals

\- authority expansion

\- diagnostic leakage

\- implicit permissions



\---



\# BETA-026 Contract Status



Frozen.



Runtime implementation, schemas, and documentation are aligned.

