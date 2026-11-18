You are helping me debug a failing checkout flow. Here are my notes and captures.

The checkout page rejects a valid coupon.

[Image 1]

Note the error banner says the code expired.

```text
curl -i https://api.example.com/checkout
```

[Image 2]

[Image 3]