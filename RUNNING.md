# Running Memri - Quick Start Guide

## 🚀 Starting the Application

### Backend (Rust)
```powershell
cd memri-app
cargo run
```

**What it does:**
- Starts the capture service at `http://127.0.0.1:8080`
- Creates SQLite database at `./memri.db`
- Begins capturing screen activity
- Serves API endpoints for frontend

### Frontend (Next.js)
```powershell
cd memri-frontend
npm run dev
```

**What it does:**
- Starts Next.js dev server at `http://localhost:3000`
- Hot-reloads on file changes
- Connects to backend API

---

## 🛑 Stopping the Application

### Stop Backend
```powershell
# Find and stop the process
Get-Process memri_backend -ErrorAction SilentlyContinue | Stop-Process -Force
```

Or press `Ctrl+C` in the terminal where it's running.

### Stop Frontend
Press `Ctrl+C` in the terminal where it's running.

---

## 🔧 Common Issues & Solutions

### Issue: "Access is denied" when running `cargo run`
**Problem:** The backend executable is locked because it's already running.

**Solution:**
```powershell
# Stop the running process
Get-Process memri_backend -ErrorAction SilentlyContinue | Stop-Process -Force

# Then rebuild
cd memri-app
cargo run
```

### Issue: Frontend can't connect to backend
**Check:**
1. Backend is running on port 8080
2. Check `NEXT_PUBLIC_MEMRI_API_URL` in frontend `.env` (should be `http://127.0.0.1:8080`)
3. CORS is enabled in backend

### Issue: Database locked
**Solution:**
```powershell
# Stop backend
Get-Process memri_backend -ErrorAction SilentlyContinue | Stop-Process -Force

# Wait a moment, then restart
cd memri-app
cargo run
```

---

## 📝 Configuration

### Backend Config (`memri-app/memri-config.toml`)
```toml
[app]
monitor_id = 0
capture_interval_ms = 2000
capture_max_interval_ms = 8000
capture_unfocused_windows = false
languages = "en"
database_url = "sqlite://./memri.db"
retention_days = 30
max_captures = 5000
image_dir = "captures"

[api]
addr = "127.0.0.1:8080"
# key = "your_api_key_here"
# anthropic_api_key = "your_anthropic_api_key_here"
```

### Frontend Config (`memri-frontend/.env.local`)
```bash
NEXT_PUBLIC_MEMRI_API_URL=http://127.0.0.1:8080
NEXT_PUBLIC_MEMRI_API_KEY=your_api_key_here
```

---

## 🧪 Development Workflow

### Full System Start
```powershell
# Terminal 1: Start backend
cd memri-app
cargo run

# Terminal 2: Start frontend
cd memri-frontend
npm run dev
```

### Just Frontend (API must be running)
```powershell
cd memri-frontend
npm run dev
```

### Rebuild After Code Changes

**Backend:**
- Rust will auto-detect changes
- Stop with `Ctrl+C` and run `cargo run` again

**Frontend:**
- Next.js hot-reloads automatically
- No restart needed for most changes

---

## 📊 Checking Status

### View Running Processes
```powershell
# Check if backend is running
Get-Process memri_backend -ErrorAction SilentlyContinue

# Check all Node processes (includes Next.js)
Get-Process node -ErrorAction SilentlyContinue
```

### View Logs

**Backend:** Logs appear in the terminal where `cargo run` was executed

**Frontend:** Logs appear in the terminal where `npm run dev` was executed

---

## 🗄️ Database Management

### View Database
```powershell
cd memri-app
sqlite3 memri.db

# Common queries
.tables                           # List all tables
SELECT COUNT(*) FROM captures;    # Count captures
SELECT * FROM captures LIMIT 10;  # View recent captures
.quit                             # Exit sqlite3
```

### Reset Database
```powershell
# Stop backend first
Get-Process memri_backend -ErrorAction SilentlyContinue | Stop-Process -Force

# Delete database
cd memri-app
Remove-Item memri.db

# Restart backend (will recreate database)
cargo run
```

---

## 🎨 Frontend Development

### View Current UI
Open `http://localhost:3000` in your browser

### Key Features Implemented
- ✅ Eden.so design system
- ✅ Cyan/purple color scheme
- ✅ Gradient message bubbles
- ✅ Auto-resizing input
- ✅ Model selector dropdown
- ✅ Timeline scrubber
- ✅ Resizable chat panel
- ✅ Smooth animations

### Making UI Changes
1. Edit files in `memri-frontend/app/`
2. Changes hot-reload automatically
3. Check browser console for errors

---

## 🐛 Debugging

### Backend Logs
Backend outputs to terminal with `tracing` crate. Look for:
- `INFO` - Normal operations
- `WARN` - Potential issues
- `ERROR` - Problems that need attention

### Frontend Logs
- Browser console (F12) - Client-side errors
- Terminal - Server-side Next.js errors
- Network tab - API call issues

### Common Debug Commands
```powershell
# Check if ports are in use
netstat -ano | findstr :8080  # Backend port
netstat -ano | findstr :3000  # Frontend port

# View all captures
cd memri-app/captures
Get-ChildItem *.png | Measure-Object | Select-Object -ExpandProperty Count
```

---

## 📦 Building for Production

### Backend
```powershell
cd memri-app
cargo build --release
# Binary will be in target/release/memri_backend.exe
```

### Frontend
```powershell
cd memri-frontend
npm run build
npm run start
```

---

## 🎉 Quick Health Check

Run this to verify everything is working:

```powershell
# Test backend
Invoke-WebRequest -Uri http://127.0.0.1:8080/captures?limit=5 -UseBasicParsing

# Test frontend (in browser)
# Navigate to http://localhost:3000
```

If both work, you're all set! 🚀

