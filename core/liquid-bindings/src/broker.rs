//! `SlotBroker` trait + `InProcessSlotBroker` Phase-2 backend.
//!
//! Implements `IMPLEMENTATION_PLAN.md §4.4` + §6.2. The broker is
//! a typed pub/sub bus over [`liquid_core::SlotValue`]s, with
//! component-to-component wiring stored declaratively so a page
//! reload can replay every subscription.
//!
//! Phase-2 ships only the in-process variant; Phase-4 (M18) adds
//! a distributed event bus behind the same trait.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use liquid_core::{LiquidError, Result, SlotName, SlotValue};

/// Maximum buffered messages per slot. A slow subscriber that
/// falls behind by more than this number of `publish`es loses the
/// oldest events (tokio's `broadcast` semantics). 256 matches the
/// §6.2 default expectation of low-frequency UI events.
pub const SLOT_BUFFER_SIZE: usize = 256;

/// One declarative wire from an output slot to an input slot
/// inside a single page. Serialised to
/// `.liquid/pages/<page_id>/bindings.json` so wiring survives a
/// page reload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SlotWiring {
    pub from: SlotName,
    pub to: SlotName,
}

/// Persistent shape — a JSON document loaded from / written to
/// `.liquid/pages/<page_id>/bindings.json` (per §6.2's
/// "wiring is replayed on page load" rule).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BindingsDocument {
    #[serde(default)]
    pub wires: Vec<SlotWiring>,
}

/// Pub/sub bus for slot values.
///
/// `publish` is fire-and-forget: it returns the count of
/// subscribers the value was delivered to (0 if nobody is
/// listening, which is fine — `publish` is the canonical event
/// emit shape and an output slot with no current subscribers is
/// a normal state).
///
/// `subscribe` returns a `BroadcastStream`-shaped receiver
/// (`tokio::sync::broadcast::Receiver`). Each subscriber gets
/// its own buffer; lagging subscribers receive
/// `broadcast::error::RecvError::Lagged(n)` and skip ahead — the
/// SDK Dart side translates this into a typed `SlotLag` event.
///
/// `wire` adds a declarative output→input edge to the broker's
/// in-process wiring table AND emits an immediate
/// `publish`-on-`from` → `publish`-on-`to` plumbing thread that
/// stays live for the broker's lifetime. `load_bindings` /
/// `save_bindings` are the JSON-document round-trip the SDK uses
/// to make wiring survive a page reload.
#[async_trait]
pub trait SlotBroker: Send + Sync {
    /// Emit `value` to every subscriber of `slot`. Returns the
    /// number of subscribers the message reached (0 if none).
    async fn publish(&self, slot: SlotName, value: SlotValue) -> Result<usize>;

    /// Subscribe to `slot`. The returned receiver delivers every
    /// future `publish`. Phase-2: no replay of historical values;
    /// Phase-4 adds last-value caching.
    async fn subscribe(&self, slot: SlotName) -> Result<broadcast::Receiver<SlotValue>>;

    /// Wire output `from` to input `to`. Every future `publish`
    /// on `from` will be republished on `to`. Idempotent — wiring
    /// the same pair twice is a no-op.
    async fn wire(&self, wiring: SlotWiring) -> Result<()>;

    /// Replace the broker's current wiring set with `doc.wires`.
    /// Used after a page load to replay persisted bindings.
    async fn load_bindings(&self, doc: BindingsDocument) -> Result<()>;

    /// Return a snapshot of the current wiring set, suitable for
    /// writing to `.liquid/pages/<page_id>/bindings.json`.
    async fn save_bindings(&self) -> Result<BindingsDocument>;
}

/// In-process implementation backed by per-slot
/// [`tokio::sync::broadcast`] channels and an in-memory wiring
/// table.
#[derive(Debug, Default)]
pub struct InProcessSlotBroker {
    state: Mutex<BrokerState>,
}

#[derive(Debug, Default)]
struct BrokerState {
    /// One broadcast channel per known slot. The `Sender` half is
    /// retained here so a late `subscribe` can pull a fresh
    /// `Receiver`; senders are cheap to clone.
    senders: HashMap<SlotName, broadcast::Sender<SlotValue>>,
    /// Active wiring edges. A `publish` on `from` triggers a
    /// `publish` on each `to` (driven by a background fan-out
    /// task spawned in [`InProcessSlotBroker::wire`]).
    wires: Vec<SlotWiring>,
}

impl InProcessSlotBroker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Acquire the broker state lock, surfacing poison as
    /// `LiquidError::InvalidInput` — matches `liquid-auth`,
    /// `liquid-permissions`, and `liquid-vcs` so a panicked thread
    /// cannot silently corrupt downstream state.
    fn lock(&self) -> Result<MutexGuard<'_, BrokerState>> {
        self.state.lock().map_err(poisoned)
    }

    /// Return (or create) the broadcast `Sender` for `slot`.
    /// Subscribers added later see future publishes; tokio's
    /// `broadcast` does not replay historical messages.
    fn sender_for(&self, slot: &SlotName) -> Result<broadcast::Sender<SlotValue>> {
        let mut g = self.lock()?;
        Ok(g.senders
            .entry(slot.clone())
            .or_insert_with(|| broadcast::channel(SLOT_BUFFER_SIZE).0)
            .clone())
    }

    /// Reject a candidate wire that would close a cycle through the
    /// existing wiring set. Direct self-loops (`from == to`) are
    /// handled by the caller; this covers multi-hop cycles like
    /// `A → B` plus a new `B → A`.
    fn would_form_cycle(wires: &[SlotWiring], candidate: &SlotWiring) -> bool {
        let mut frontier: Vec<&SlotName> = vec![&candidate.to];
        while let Some(node) = frontier.pop() {
            if node == &candidate.from {
                return true;
            }
            for w in wires.iter().filter(|w| &w.from == node) {
                frontier.push(&w.to);
            }
        }
        false
    }
}

fn poisoned<T>(_: PoisonError<T>) -> LiquidError {
    LiquidError::InvalidInput("slot broker state lock poisoned".into())
}

#[async_trait]
impl SlotBroker for InProcessSlotBroker {
    async fn publish(&self, slot: SlotName, value: SlotValue) -> Result<usize> {
        let sender = self.sender_for(&slot)?;
        // `broadcast::Sender::send` returns `Err(SendError(v))`
        // when there are no active receivers; that is normal +
        // not a project error — surface 0.
        let direct = sender.send(value.clone()).unwrap_or(0);

        // Fan-out to wired downstream slots. Snapshot the wires
        // list under the lock so the broker doesn't deadlock if
        // a downstream `publish` recurses.
        let wires: Vec<SlotWiring> = self
            .lock()?
            .wires
            .iter()
            .filter(|w| w.from == slot)
            .cloned()
            .collect();
        let mut downstream: usize = 0;
        for w in wires {
            let s = self.sender_for(&w.to)?;
            downstream = downstream.saturating_add(s.send(value.clone()).unwrap_or(0));
        }
        Ok(direct.saturating_add(downstream))
    }

    async fn subscribe(&self, slot: SlotName) -> Result<broadcast::Receiver<SlotValue>> {
        Ok(self.sender_for(&slot)?.subscribe())
    }

    async fn wire(&self, wiring: SlotWiring) -> Result<()> {
        if wiring.from == wiring.to {
            return Err(LiquidError::InvalidInput(format!(
                "self-wiring is not allowed: {}",
                wiring.from
            )));
        }
        let mut g = self.lock()?;
        if g.wires.contains(&wiring) {
            return Ok(());
        }
        if Self::would_form_cycle(&g.wires, &wiring) {
            return Err(LiquidError::InvalidInput(format!(
                "wiring would form a cycle: {} → {}",
                wiring.from, wiring.to
            )));
        }
        g.wires.push(wiring);
        Ok(())
    }

    async fn load_bindings(&self, doc: BindingsDocument) -> Result<()> {
        // Reject self-wires up-front so a malformed document does
        // not silently corrupt the in-memory state.
        for w in &doc.wires {
            if w.from == w.to {
                return Err(LiquidError::InvalidInput(format!(
                    "self-wiring in bindings document: {}",
                    w.from
                )));
            }
        }
        // Reject documents whose wires close a cycle as well, so
        // the on-disk shape stays consistent with what `wire`
        // accepts at runtime.
        let mut seen: Vec<SlotWiring> = Vec::with_capacity(doc.wires.len());
        for w in &doc.wires {
            if Self::would_form_cycle(&seen, w) {
                return Err(LiquidError::InvalidInput(format!(
                    "bindings document closes a cycle at {} → {}",
                    w.from, w.to
                )));
            }
            seen.push(w.clone());
        }
        let mut g = self.lock()?;
        g.wires = doc.wires;
        Ok(())
    }

    async fn save_bindings(&self) -> Result<BindingsDocument> {
        let g = self.lock()?;
        Ok(BindingsDocument {
            wires: g.wires.clone(),
        })
    }
}

/// Convenience: `Arc<dyn SlotBroker>` shape every consumer can
/// hold. The bridge will hand one of these to FFI callers.
pub type SharedBroker = Arc<dyn SlotBroker>;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    fn slot(s: &str) -> SlotName {
        SlotName::new(s).expect("test slot")
    }

    #[tokio::test]
    async fn publish_with_no_subscribers_returns_zero() {
        let b = InProcessSlotBroker::new();
        let n = b
            .publish(slot("ns:out"), SlotValue::Num(1.0))
            .await
            .expect("publish");
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn subscribe_then_publish_delivers_one_message() {
        let b = InProcessSlotBroker::new();
        let mut rx = b.subscribe(slot("ns:out")).await.expect("subscribe");
        let n = b
            .publish(slot("ns:out"), SlotValue::Num(42.0))
            .await
            .expect("publish");
        assert_eq!(n, 1, "exactly one receiver heard the publish");
        let got = timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("recv timeout")
            .expect("recv ok");
        assert_eq!(got, SlotValue::Num(42.0));
    }

    #[tokio::test]
    async fn two_subscribers_each_get_their_own_copy() {
        let b = InProcessSlotBroker::new();
        let mut a = b.subscribe(slot("ns:out")).await.expect("a");
        let mut c = b.subscribe(slot("ns:out")).await.expect("c");
        let n = b
            .publish(slot("ns:out"), SlotValue::Bool(true))
            .await
            .expect("publish");
        assert_eq!(n, 2);
        assert_eq!(
            timeout(Duration::from_millis(100), a.recv())
                .await
                .unwrap()
                .unwrap(),
            SlotValue::Bool(true)
        );
        assert_eq!(
            timeout(Duration::from_millis(100), c.recv())
                .await
                .unwrap()
                .unwrap(),
            SlotValue::Bool(true)
        );
    }

    #[tokio::test]
    async fn wire_routes_publishes_to_downstream_subscribers() {
        let b = InProcessSlotBroker::new();
        let mut downstream = b.subscribe(slot("chart:data")).await.expect("downstream");
        b.wire(SlotWiring {
            from: slot("sheet:selectedRange"),
            to: slot("chart:data"),
        })
        .await
        .expect("wire");
        let n = b
            .publish(slot("sheet:selectedRange"), SlotValue::Str("A1:B2".into()))
            .await
            .expect("publish");
        // 0 direct (no subscriber on `sheet:selectedRange`) + 1
        // downstream via the wire = 1.
        assert_eq!(n, 1);
        let got = timeout(Duration::from_millis(100), downstream.recv())
            .await
            .expect("recv timeout")
            .expect("recv ok");
        assert_eq!(got, SlotValue::Str("A1:B2".into()));
    }

    #[tokio::test]
    async fn wire_rejects_self_loop() {
        let b = InProcessSlotBroker::new();
        let err = b
            .wire(SlotWiring {
                from: slot("ns:loop"),
                to: slot("ns:loop"),
            })
            .await
            .expect_err("self-wire must fail");
        assert!(matches!(err, LiquidError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn wire_is_idempotent() {
        let b = InProcessSlotBroker::new();
        let w = SlotWiring {
            from: slot("ns:a"),
            to: slot("ns:b"),
        };
        b.wire(w.clone()).await.expect("ok");
        b.wire(w.clone()).await.expect("ok");
        let doc = b.save_bindings().await.expect("save");
        assert_eq!(doc.wires.len(), 1, "dup wire must not double-register");
    }

    #[tokio::test]
    async fn save_then_load_round_trips_the_wiring_document() {
        let b1 = InProcessSlotBroker::new();
        b1.wire(SlotWiring {
            from: slot("a:out"),
            to: slot("b:in"),
        })
        .await
        .expect("ok");
        b1.wire(SlotWiring {
            from: slot("c:out"),
            to: slot("d:in"),
        })
        .await
        .expect("ok");
        let doc = b1.save_bindings().await.expect("save");
        assert_eq!(doc.wires.len(), 2);

        // Replay into a fresh broker (simulates page reload).
        let b2 = InProcessSlotBroker::new();
        b2.load_bindings(doc.clone()).await.expect("load");
        let echoed = b2.save_bindings().await.expect("save");
        assert_eq!(echoed.wires, doc.wires);

        // Wiring is live after reload — publish on a:out reaches
        // a subscriber on b:in.
        let mut rx = b2.subscribe(slot("b:in")).await.expect("subscribe");
        let n = b2
            .publish(slot("a:out"), SlotValue::Num(7.0))
            .await
            .expect("publish");
        assert!(n >= 1);
        let got = timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("timeout")
            .expect("recv");
        assert_eq!(got, SlotValue::Num(7.0));
    }

    #[tokio::test]
    async fn wire_rejects_multi_hop_cycle() {
        let b = InProcessSlotBroker::new();
        b.wire(SlotWiring {
            from: slot("a:out"),
            to: slot("b:in"),
        })
        .await
        .expect("first wire ok");
        let err = b
            .wire(SlotWiring {
                from: slot("b:in"),
                to: slot("a:out"),
            })
            .await
            .expect_err("second wire would close a cycle");
        assert!(matches!(err, LiquidError::InvalidInput(_)));
        let doc = b.save_bindings().await.expect("save");
        assert_eq!(doc.wires.len(), 1, "cycle must not be persisted");
    }

    #[tokio::test]
    async fn wire_rejects_three_hop_cycle() {
        // A → B → C → A — exercises the multi-intermediate-node
        // traversal in `would_form_cycle` that the 2-hop test
        // cannot reach (DFS depth = 1).
        let b = InProcessSlotBroker::new();
        b.wire(SlotWiring {
            from: slot("a:out"),
            to: slot("b:in"),
        })
        .await
        .expect("a→b ok");
        b.wire(SlotWiring {
            from: slot("b:in"),
            to: slot("c:in"),
        })
        .await
        .expect("b→c ok");
        let err = b
            .wire(SlotWiring {
                from: slot("c:in"),
                to: slot("a:out"),
            })
            .await
            .expect_err("c→a would close a 3-hop cycle");
        assert!(matches!(err, LiquidError::InvalidInput(_)));
        let doc = b.save_bindings().await.expect("save");
        assert_eq!(
            doc.wires.len(),
            2,
            "the two acyclic legs survive; the cycle-closer is rejected"
        );
    }

    #[tokio::test]
    async fn load_bindings_rejects_multi_hop_cycle() {
        let b = InProcessSlotBroker::new();
        let bad = BindingsDocument {
            wires: vec![
                SlotWiring {
                    from: slot("a:out"),
                    to: slot("b:in"),
                },
                SlotWiring {
                    from: slot("b:in"),
                    to: slot("a:out"),
                },
            ],
        };
        let err = b.load_bindings(bad).await.expect_err("must reject");
        assert!(matches!(err, LiquidError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn load_bindings_rejects_self_wires() {
        let b = InProcessSlotBroker::new();
        let bad = BindingsDocument {
            wires: vec![SlotWiring {
                from: slot("ns:bad"),
                to: slot("ns:bad"),
            }],
        };
        let err = b.load_bindings(bad).await.expect_err("must reject");
        assert!(matches!(err, LiquidError::InvalidInput(_)));
    }

    #[test]
    fn bindings_document_round_trips_json() {
        let doc = BindingsDocument {
            wires: vec![SlotWiring {
                from: slot("a:b"),
                to: slot("c:d"),
            }],
        };
        let json = serde_json::to_string(&doc).expect("ser");
        let back: BindingsDocument = serde_json::from_str(&json).expect("de");
        assert_eq!(back, doc);
    }
}
