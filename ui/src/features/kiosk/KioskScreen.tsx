import { useEffect, useState, useRef, useCallback } from 'react';
import { Localized } from '@/components/Localized';
import { listProducts, listCategories } from '@/api/products';
import type { ProductDto, CategoryDto } from '@/api/products';
import './KioskScreen.css';

const IDLE_TIMEOUT_MS = 60000;

interface CartItem {
  product: ProductDto;
  qty: number;
}

export default function KioskScreen() {
  const [products, setProducts] = useState<ProductDto[]>([]);
  const [categories, setCategories] = useState<CategoryDto[]>([]);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const [cart, setCart] = useState<CartItem[]>([]);
  const [checkout, setCheckout] = useState(false);
  const [idle, setIdle] = useState(false);
  const idleTimer = useRef<ReturnType<typeof setTimeout>>();

  const resetIdle = useCallback(() => {
    if (idle) setIdle(false);
    clearTimeout(idleTimer.current);
    idleTimer.current = setTimeout(() => setIdle(true), IDLE_TIMEOUT_MS);
  }, [idle]);

  useEffect(() => {
    listProducts().then(setProducts);
    listCategories().then(setCategories);
    idleTimer.current = setTimeout(() => setIdle(true), IDLE_TIMEOUT_MS);
    return () => clearTimeout(idleTimer.current);
  }, []);

  useEffect(() => {
    const handler = () => resetIdle();
    window.addEventListener('touchstart', handler);
    window.addEventListener('click', handler);
    window.addEventListener('keydown', handler);
    return () => {
      window.removeEventListener('touchstart', handler);
      window.removeEventListener('click', handler);
      window.removeEventListener('keydown', handler);
    };
  }, [resetIdle]);

  const filtered = activeCategory
    ? products.filter((p) => p.category === activeCategory)
    : products;

  const addToCart = (product: ProductDto) => {
    setCart((prev) => {
      const existing = prev.find((c) => c.product.sku === product.sku);
      if (existing) {
        return prev.map((c) =>
          c.product.sku === product.sku
            ? { ...c, qty: c.qty + 1 }
            : c
        );
      }
      return [...prev, { product, qty: 1 }];
    });
    resetIdle();
  };

  const updateQty = (sku: string, delta: number) => {
    setCart((prev) => {
      const next = prev
        .map((c) =>
          c.product.sku === sku
            ? { ...c, qty: Math.max(0, c.qty + delta) }
            : c
        )
        .filter((c) => c.qty > 0);
      return next;
    });
  };

  const totalMinor = cart.reduce((s, c) => s + c.product.price.minor_units * c.qty, 0);

  if (idle) {
    return (
      <div className="kiosk-attract" role="button" aria-label="Kiosk attract screen" tabIndex={0} onClick={resetIdle} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') resetIdle(); }}>
        <div className="kiosk-attract-content">
          <h1 className="kiosk-attract-title">OZ-POS</h1>
          <p className="kiosk-attract-subtitle"><Localized id="kiosk-tap-to-start">Tap to start</Localized></p>
        </div>
      </div>
    );
  }

  if (checkout) {
    return (
      <div className="kiosk-checkout" role="region" aria-label="Checkout">
        <h2><Localized id="kiosk-checkout-title">Checkout</Localized></h2>
        <div className="kiosk-checkout-items">
          {cart.map((c) => (
            <div key={c.product.sku} className="kiosk-checkout-row">
              <span>{c.product.name} &times; {c.qty}</span>
              <span>${((c.product.price.minor_units * c.qty) / 100).toFixed(2)}</span>
            </div>
          ))}
        </div>
        <div className="kiosk-checkout-total">
          <span><Localized id="kiosk-total">Total</Localized></span>
          <span>${(totalMinor / 100).toFixed(2)}</span>
        </div>
        <button className="kiosk-checkout-pay" onClick={() => {
          alert('Payment processed! (simulated)');
          setCart([]);
          setCheckout(false);
        }} aria-label="Pay">
          <Localized id="kiosk-pay">Pay</Localized>
        </button>
        <button className="kiosk-checkout-back" onClick={() => setCheckout(false)} aria-label="Back">
          <Localized id="back">Back</Localized>
        </button>
      </div>
    );
  }

  return (
    <div className="kiosk" role="region" aria-label="Self-service kiosk">
      <div className="kiosk-categories" role="tablist" aria-label="Categories">
        <button
          className={`kiosk-cat-chip ${activeCategory === null ? 'active' : ''}`}
          onClick={() => setActiveCategory(null)}
          role="tab"
          aria-selected={activeCategory === null}
        >
          <Localized id="kiosk-all">All</Localized>
        </button>
        {categories.map((cat) => (
          <button
            key={cat.id}
            className={`kiosk-cat-chip ${activeCategory === cat.id ? 'active' : ''}`}
            onClick={() => setActiveCategory(cat.id)}
            role="tab"
            aria-selected={activeCategory === cat.id}
          >
            {cat.name}
          </button>
        ))}
      </div>

      <div className="kiosk-grid" role="list" aria-label="Products">
        {filtered.map((p) => (
          <button
            key={p.sku}
            className="kiosk-product-card"
            onClick={() => addToCart(p)}
            aria-label={`${p.name}, $${(p.price.minor_units / 100).toFixed(2)}`}
          >
            <span className="kiosk-product-name">{p.name}</span>
            <span className="kiosk-product-price">${(p.price.minor_units / 100).toFixed(2)}</span>
            {p.stock_qty !== null && p.stock_qty <= 5 && (
              <span className="kiosk-stock-badge">{p.stock_qty} left</span>
            )}
          </button>
        ))}
      </div>

      {cart.length > 0 && (
        <div className="kiosk-cart" role="region" aria-label="Cart">
          <div className="kiosk-cart-items">
            {cart.map((c) => (
              <div key={c.product.sku} className="kiosk-cart-item">
                <span className="kiosk-cart-name">{c.product.name}</span>
                <div className="kiosk-cart-controls">
                  <button onClick={() => updateQty(c.product.sku, -1)} aria-label="Decrease">&minus;</button>
                  <span>{c.qty}</span>
                  <button onClick={() => updateQty(c.product.sku, 1)} aria-label="Increase">+</button>
                </div>
                <span className="kiosk-cart-price">${((c.product.price.minor_units * c.qty) / 100).toFixed(2)}</span>
              </div>
            ))}
          </div>
          <div className="kiosk-cart-total">
            <span><Localized id="kiosk-total">Total</Localized></span>
            <span>${(totalMinor / 100).toFixed(2)}</span>
          </div>
          <button className="kiosk-checkout-btn" onClick={() => setCheckout(true)} aria-label="Checkout">
            <Localized id="kiosk-checkout">Checkout</Localized>
          </button>
        </div>
      )}
    </div>
  );
}
