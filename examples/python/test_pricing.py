from pricing import discounted_price
import unittest


class Pricing(unittest.TestCase):
    def test_basic(self):
        self.assertEqual(discounted_price(1000, 10), 900)

    def test_floor(self):
        self.assertEqual(discounted_price(99, 50), 49)


if __name__ == "__main__":
    unittest.main()
