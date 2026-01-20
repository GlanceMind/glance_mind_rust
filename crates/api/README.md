# 🧠 Glance Mind API

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Axum](https://img.shields.io/badge/axum-0.6-orange?style=for-the-badge)
![PostgreSQL](https://img.shields.io/badge/postgres-%23316192.svg?style=for-the-badge&logo=postgresql&logoColor=white)
![Redis](https://img.shields.io/badge/redis-%23DD0031.svg?style=for-the-badge&logo=redis&logoColor=white)
![Docker](https://img.shields.io/badge/docker-%230db7ed.svg?style=for-the-badge&logo=docker&logoColor=white)

**Glance Mind API** is the high-performance backend core for the Glance Mind platform, designed to orchestrate AI-driven social media automation campaigns. It handles business logic, data persistence, and coordination between the Web Portal and the Job Scheduler.

## 🚀 Core Features

-   **Campaign Management**: Create, configure, and monitor social media campaigns.
-   **User & Wallet System**: Secure authentication (JWT), RBAC, and credit-based billing.
-   **AI Orchestration**: Integration with LLMs for dynamic content generation and engagement.
-   **Scheduler Integration**: Async task dispatching via Redis for high-precision job execution.
-   **Extensible Architecture**: Built with Rust (Axum) and Hexagonal Architecture (Service/Repository layers).

## 🛠 Tech Stack

-   **Language**: Rust (Edition 2021)
-   **Framework**: [Axum](https://github.com/tokio-rs/axum)
-   **Database**: PostgreSQL (via Diesel ORM)
-   **Cache/Queue**: Redis
-   **Authentication**: JWT & Argon2

## 📦 Getting Started

### Prerequisites

-   Rust (latest stable)
-   Docker & Docker Compose (for DB/Redis)

### Installation

1.  **Clone the repository**
    ```bash
    git clone https://github.com/your-org/glance-mind-api.git
    cd glance-mind-api
    ```

2.  **Setup Environment**
    Copy `.env.example` to `.env` and configure your database credentials.
    ```bash
    cp .env.example .env
    ```

3.  **Start Infrastructure**
    ```bash
    docker-compose up -d db redis
    ```

4.  **Run Migrations**
    ```bash
    diesel migration run
    ```

5.  **Run Server**
    ```bash
    cargo run
    ```

# glance_mind_api
# glance_mind_api
