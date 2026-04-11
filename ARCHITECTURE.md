# QuantumCart Architecture

QuantumCart is a next-generation decentralized e-commerce platform designed to be a self-sustaining retail grid. It consists of:
1. An AI-driven inventory prediction system & high-speed Rust backend.
2. A headless SvelteKit storefront with 3D product rendering.
3. An automated drop-shipping WordPress/WooCommerce plugin for seamless product import.

## 1. Database Schema (PostgreSQL Cluster)

To handle 10,000 transactions a minute and provide high fault tolerance, we use a highly available PostgreSQL cluster setup.

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

### `transactions` Table
```sql
CREATE TABLE transactions (
    transaction_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID REFERENCES products(product_id),
    buyer_id UUID,
    vendor_id UUID REFERENCES vendors(vendor_id),
    quantity INTEGER NOT NULL,
    total_amount DECIMAL(10, 2) NOT NULL,
    status VARCHAR(50) DEFAULT 'PENDING', -- PENDING, COMPLETED, FAILED
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_transactions_vendor ON transactions(vendor_id);
CREATE INDEX idx_transactions_created_at ON transactions(created_at);
```

## 2. API Routing Table (Rust Backend)

The Rust high-speed backend provides RESTful APIs for the frontend and plugins.

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/v1/products` | Retrieve a list of products (paginated, cached) |
| `GET` | `/api/v1/products/:id` | Get details for a specific product |
| `GET` | `/api/v1/inventory/:product_id` | Get real-time inventory level (serves from Redis) |
| `POST` | `/api/v1/checkout` | Process a transaction/checkout |
| `POST` | `/api/v1/plugin/import` | For WP Plugin to import product details to their store |
| `POST` | `/api/v1/webhooks/vendor` | Receive inventory updates/orders from vendors |

## 3. Redis Caching Strategy

To achieve sub-100ms load times and handle 10,000 transactions a minute:

1. **Inventory Caching:**
   - **Key Pattern:** `inventory:{product_id}`
   - **Value:** Integer (stock level)
   - **TTL:** 10 seconds (Updates happen every 5 seconds via scraper)
   - **Strategy:** Read-Through and Write-Through caching. The Rust backend writes to Redis first, which then queues an async update to PostgreSQL.

2. **Product Catalog Caching:**
   - **Key Pattern:** `product:{product_id}`
   - **Value:** JSON serialized product details.
   - **TTL:** 1 Hour. Invalidation occurs on product update.

3. **Rate Limiting / Scraping Buffer:**
   - Use Redis sets or sorted sets to manage scraping queues and respect supplier API rate limits.
   - Lock keys `scraper_lock:{vendor_id}` to prevent concurrent overlapping scrapes.

## 4. Webhook Payload Structures

Communication between the Rust Backend, SvelteKit Frontend, and WP Plugin uses event-driven webhooks.

### 4.1. Inventory Update (Rust -> WP Plugin)
Sent when inventory levels change or AI predicts a stock-out.
```json
{
  "event": "inventory.updated",
  "product_id": "uuid-string",
  "new_stock_level": 150,
  "timestamp": "2023-10-27T10:00:00Z"
}
```

### 4.2. Transaction Completed (Frontend -> Rust -> WP Plugin)
Sent when an order is successfully processed.
```json
{
  "event": "transaction.completed",
  "transaction_id": "uuid-string",
  "product_id": "uuid-string",
  "quantity": 2,
  "total_amount": 59.98,
  "buyer_info": {
    "name": "John Doe",
    "email": "john@example.com"
  },
  "timestamp": "2023-10-27T10:05:00Z"
}
```

### 4.3. Product Import Request (WP Plugin -> Rust)
Requested by the WP Plugin to import a product to the external vendor.
```json
{
  "action": "product.import",
  "vendor_api_key": "v_api_12345",
  "product_id": "uuid-string",
  "target_store_url": "https://vendor-store.com"
}
```