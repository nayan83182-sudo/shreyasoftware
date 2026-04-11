# QuantumCart Enterprise Architecture

QuantumCart is an enterprise-grade, highly scalable decentralized e-commerce platform designed to be a self-sustaining retail grid.

## 1. Enterprise Architecture Patterns

### 1.1 High-Availability & Global Scale
- **PostgreSQL Cluster:** Active-Active multi-region deployment using Citus or CockroachDB concepts for globally distributed consistency.
- **Redis Enterprise:** Clustered caching layer for sub-100ms latency globally.
- **CDN Edge Rendering:** SvelteKit frontend distributed globally, rendering 3D models at the edge.

### 1.2 CQRS & Event Sourcing (Transactions)
- **Command Query Responsibility Segregation (CQRS):** Write operations (orders) are processed via Apache Kafka/Redpanda event streams. Read operations (inventory, products) are served from optimized read-replicas and Redis clusters.
- **Event Sourcing:** Transactions are immutable events. State (like current inventory) is a materialized view derived from the event log.

### 1.3 Zero-Trust Security & Observability
- **Security:** Strict mutual TLS (mTLS) between microservices. JWT-based stateless authentication. WAF (Web Application Firewall) at the edge.
- **Observability:** OpenTelemetry (OTEL) instrumented across Rust, SvelteKit, and PHP plugins. Distributed tracing and structured logging shipped to Elasticsearch/Datadog.

## 2. Database Schema (PostgreSQL Partitioned Cluster)

To handle 10,000+ transactions a minute, the `transactions` table is partitioned by date, and audit trails are implemented.

### `vendors` Table
```sql
CREATE TABLE vendors (
    vendor_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    api_key VARCHAR(255) UNIQUE NOT NULL,
    webhook_url VARCHAR(512),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
```

### `products` Table
```sql
CREATE TABLE products (
    product_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendor_id UUID REFERENCES vendors(vendor_id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    price DECIMAL(10, 2) NOT NULL,
    currency VARCHAR(3) DEFAULT 'USD',
    three_d_model_url VARCHAR(512),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_products_vendor ON products(vendor_id);
```

### `inventory` Table
```sql
CREATE TABLE inventory (
    inventory_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID REFERENCES products(product_id) ON DELETE CASCADE UNIQUE,
    stock_level INTEGER NOT NULL DEFAULT 0,
    predicted_demand INTEGER DEFAULT 0, -- AI predicted demand
    last_scraped_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_inventory_product ON inventory(product_id);
```

### `transactions` Table (Partitioned)
```sql
CREATE TABLE transactions (
    transaction_id UUID DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL,
    buyer_id UUID,
    vendor_id UUID NOT NULL,
    quantity INTEGER NOT NULL,
    total_amount DECIMAL(10, 2) NOT NULL,
    status VARCHAR(50) DEFAULT 'PENDING',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (transaction_id, created_at)
) PARTITION BY RANGE (created_at);

-- Example Partition
CREATE TABLE transactions_y2023m10 PARTITION OF transactions
    FOR VALUES FROM ('2023-10-01') TO ('2023-11-01');
```

## 3. API Routing Table (Rust Backend)

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/v1/products` | Retrieve a list of products (paginated, cached) |
| `GET` | `/api/v1/products/:id` | Get details for a specific product |
| `GET` | `/api/v1/inventory/:product_id` | Get real-time inventory level (serves from Redis) |
| `POST` | `/api/v1/checkout` | CQRS Command: Enqueue transaction event |
| `POST` | `/api/v1/plugin/import` | For WP Plugin to import product details |
| `POST` | `/api/v1/webhooks/vendor` | Receive inventory updates/orders from vendors |
| `GET` | `/health` | Kubernetes readiness/liveness probe |

## 4. Redis Caching Strategy

1. **Inventory Caching (Write-Through):**
   - **Key Pattern:** `inventory:{product_id}`
   - **TTL:** 10 seconds.
2. **Product Catalog Caching (Read-Through):**
   - **Key Pattern:** `product:{product_id}`
   - **TTL:** 1 Hour. Invalidation occurs on product update via Event stream.
3. **Rate Limiting (Token Bucket):**
   - Key: `rate_limit:{ip}:{endpoint}`. Handled via Redis Lua scripts to ensure atomicity at 10k TPS.

## 5. Webhook Payload Structures

*(JSON Payloads for Event-Driven Communication remain standard as per V1, enhanced with trace IDs)*
```json
{
  "event": "transaction.completed",
  "transaction_id": "uuid-string",
  "trace_id": "otel-trace-uuid",
  "product_id": "uuid-string",
  "quantity": 2,
  "timestamp": "2023-10-27T10:05:00Z"
}
```