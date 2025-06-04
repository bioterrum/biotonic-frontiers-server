# 🚀 Biotonic Frontiers Server

Welcome to the **Biotonic Frontiers Server** repository! This README will guide you through setting up a fresh server on **Debian 12 / Ubuntu 22.04** so you can get the backend up and running quickly. Let’s dive in! ✨

---

## 📋 Table of Contents

1. [Prerequisites](#-prerequisites)
2. [Environment Setup](#-environment-setup)
3. [Installing Dependencies](#-installing-dependencies)
4. [Database Setup](#-database-setup)
5. [Clone & Configure Repository](#-clone--configure-repository)
6. [Run Database Migrations](#-run-database-migrations)
7. [Run the Server](#-run-the-server)
8. [Common Commands](#-common-commands)
9. [Troubleshooting & Tips](#-troubleshooting--tips)
10. [Contributing](#-contributing)
11. [License](#-license)

---

## 🔧 Prerequisites

* Fresh Debian 12 / Ubuntu 22.04
* Sudo privileges
* Internet connection

---

## 🌱 Environment Setup

```bash
sudo apt update && sudo apt upgrade -y
```

---

## 🛠️ Installing Dependencies

**PostgreSQL & Redis:**

```bash
sudo apt install -y postgresql redis
```

**Rust (via `rustup`):**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Node.js (Next.js):**

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo bash -
sudo apt-get install -y nodejs
```

**`sqlx-cli` (Postgres):**

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

---

## 🗄️ Database Setup

Set up PostgreSQL:

```bash
sudo passwd postgres
su - postgres
psql
ALTER USER postgres WITH ENCRYPTED PASSWORD 'secure-postgres-pass';
CREATE USER yourusername WITH ENCRYPTED PASSWORD 'user-secure-pass';
ALTER USER yourusername CREATEDB;
\q
exit
```

---

## 📂 Clone & Configure Repository

```bash
git clone https://github.com/bioterrum/biotonic-frontiers-server.git
cd biotonic-frontiers-server
```

Create `.env`:

```env
DATABASE_URL=postgres://yourusername:user-secure-pass@localhost/biotonic_db
REDIS_URL=redis://127.0.0.1:6379
```

---

## 📑 Run Database Migrations

```bash
sqlx database create
sqlx migrate run
```

---

## ▶️ Run the Server

```bash
cd server
cargo run
```

Server listens on `http://127.0.0.1:8080`

---

## 🏃 Common Commands

```bash
# Database
sqlx database create
sqlx migrate run

# Rust server
cargo fmt
cargo clippy
cargo test
cargo build --release
./target/release/biotonic-server

# Reset DB (⚠️ destructive!)
sqlx database reset
sqlx migrate run

# Redis
redis-cli ping  # PONG
```

---

## 🛠️ Troubleshooting & Tips

* Check PostgreSQL: `sudo systemctl status postgresql`
* Check paths for cargo binaries
* Resolve port conflicts by adjusting environment vars

---

## 🤝 Contributing

* Fork repo
* Create branch, commit
* Push, create Pull Request

---

## 📜 License

MIT License. See [LICENSE](LICENSE) for details.

---

🌿 **Happy coding and see you in Bioterrum!** 🧬

