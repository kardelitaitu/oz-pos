const API = (window.__OZ_CONFIG__ && window.__OZ_CONFIG__.licenseApiUrl) || 'https://license.ozpos.my.id';
    let useOtp = false;
    let otpTimer = null;

    function showError(msg) { const e = document.getElementById('error-msg'); e.textContent = msg; e.classList.remove('hidden'); }
    function hideError() { document.getElementById('error-msg').classList.add('hidden'); }
    function setLoading(l) { document.getElementById('login-form').style.display = l ? 'none' : 'block'; document.getElementById('loading-state').style.display = l ? 'block' : 'none'; }

    function toggleOtp() {
      useOtp = !useOtp;
      document.getElementById('otp-row').classList.toggle('hidden', !useOtp);
      const toggle = document.getElementById('otp-toggle');
      toggle.textContent = useOtp ? 'Sign in with password instead' : 'Send a code to my email instead';
      document.getElementById('login-btn').textContent = useOtp ? 'Send code' : 'Sign in';
      hideError();
      if (useOtp) { document.getElementById('password').style.display = 'none'; } 
      else { document.getElementById('password').style.display = 'block'; }
    }

    function startOtpCooldown() {
      let sec = 60;
      const toggle = document.getElementById('otp-toggle');
      const cd = document.getElementById('otp-cooldown');
      cd.classList.remove('hidden');
      toggle.style.display = 'none';
      cd.textContent = `Resend code in ${sec}s`;
      if (otpTimer) clearInterval(otpTimer);
      otpTimer = setInterval(() => {
        sec--;
        if (sec <= 0) { clearInterval(otpTimer); cd.classList.add('hidden'); toggle.style.display = 'block'; }
        else { cd.textContent = `Resend code in ${sec}s`; }
      }, 1000);
    }

    // ── Exchange token for a one-time code (hardening F1) ──────────
    // The session token never goes in a URL: exchange it for a short-lived
    // single-use code, redirect with ?code=, and the Worker consumes the
    // code to set the httpOnly cookie.
    async function exchangeForCode(token) {
      const res = await fetch(API + '/api/v1/web/exchange-issue', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ' + token }
      });
      if (!res.ok) throw new Error('exchange failed');
      const body = await res.json();
      window.location.href = '/?code=' + encodeURIComponent(body.code);
    }

    async function handleLogin() {
      hideError();
      const email = document.getElementById('email').value.trim();
      if (!email) { showError('Enter your email'); return; }

      if (useOtp) {
        const code = document.getElementById('otp-code').value.trim();
        if (!code && document.getElementById('otp-row').classList.contains('hidden')) {
          // First step: request OTP
          setLoading(true);
          try {
            const res = await fetch(API + '/api/v1/web/request-otp', {
              method: 'POST', headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ email })
            });
            const body = await res.json();
            if (res.status === 200) {
              document.getElementById('otp-row').classList.remove('hidden');
              document.getElementById('login-btn').textContent = 'Verify code';
              startOtpCooldown();
              setLoading(false);
              return;
            }
            if (res.status === 429) {
              showLockoutCountdown(body.retry_after || 60);
              setLoading(false);
              return;
            }
            if (res.status === 503) { showError('Email delivery is not configured'); setLoading(false); return; }
            showError(body.error || 'Failed to send code');
          } catch { showError('Could not connect to server'); }
          setLoading(false);
          return;
        }
        // Second step: verify OTP
        if (!code) { showError('Enter the 6-digit code'); return; }
        setLoading(true);
        try {
          const res = await fetch(API + '/api/v1/web/verify-otp', {
            method: 'POST', headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email, code })
          });
          const body = await res.json();
          if (res.status === 200 && body.token) {
            await exchangeForCode(body.token);
            return;
          }
          if (res.status === 429) {
            const body2 = await res.json();
            showLockoutCountdown(body2.retry_after || 60);
            setLoading(false);
            return;
          }
          showError('Invalid or expired code');
        } catch { showError('Could not connect to server'); }
        setLoading(false);
        return;
      }

      // Password login
      const password = document.getElementById('password').value;
      if (!password) { showError('Enter your password'); return; }
      setLoading(true);
      try {
        const res = await fetch(API + '/api/v1/web/login', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email, password })
        });
        const body = await res.json();
        if (res.status === 200 && body.token) {
          await exchangeForCode(body.token);
          return;
        }
        if (res.status === 429) {
          const retryAfter = body.retry_after || 60;
          showLockoutCountdown(retryAfter);
          setLoading(false);
          return;
        }
        if (res.status === 429 && body.retry_after === undefined) { showError('Too many attempts — try again later'); setLoading(false); return; }
        showError('Invalid email or password');
      } catch { showError('Could not connect to server'); }
      setLoading(false);
    }

    // ── Lockout countdown (429 with retry_after) ─────────────────
    function showLockoutCountdown(seconds) {
      const btn = document.getElementById('login-btn');
      btn.disabled = true;
      let remaining = seconds;
      btn.textContent = `Try again in ${remaining}s`;
      const timer = setInterval(() => {
        remaining--;
        if (remaining <= 0) {
          clearInterval(timer);
          btn.disabled = false;
          btn.textContent = useOtp ? 'Send code' : 'Sign in';
          return;
        }
        btn.textContent = `Try again in ${remaining}s`;
      }, 1000);
    }
  
// ── Event wiring (no inline handlers — strict CSP) ──────
document.getElementById('login-btn').addEventListener('click', handleLogin);
document.getElementById('otp-toggle').addEventListener('click', toggleOtp);

