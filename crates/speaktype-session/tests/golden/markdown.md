You are helping me debug a failing checkout flow. Here are my notes and captures.

The checkout page rejects a valid coupon.

![Mark 1](images/01-mark.png)

Note the error banner says the code expired.

```text
curl -i https://api.example.com/checkout
```

![Mark 2](images/02-mark.png)

![Screenshot 1](images/01-screenshot.png)