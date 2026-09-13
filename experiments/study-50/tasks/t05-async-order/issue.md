Issue t05: bump(price, pct) must return price increased by pct% (bump(100,10)==110).
Currently returns 10. Deliverable: correct formula, keep saturating on overflow, add test bump(99,50)==148 (floor).
