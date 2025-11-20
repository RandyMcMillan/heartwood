# Use the Python requests library for fetching a page.
import logging
import requests


# Step: remember the URL of a web site we want to verify.
#
# The URL is extracted from the step using a pattern in the bindings file. The
# function gets the value via a keyword argument.
#
# Each step function also gets a "context" variable, which starts out as an
# empty dict-like object for each scenario. The same context variable is given
# to each function for the scenario. It's a convenient way to remember data
# between steps.
def remember_url(ctx, url=None):
    ctx["url"] = url


# Step: fetch the remembered URL.
def fetch_url(ctx):
    url = ctx["url"]
    r = requests.get(url)
    logging.debug(f"GET status {r.status_code}")
    assert r.ok
    ctx["page"] = r.text


# Step: page contains some specific text.
def page_contains(ctx, text=None):
    assert text in ctx["page"]


# Step: page does NOT contain some specific text.
def page_doesnt_contain(ctx, text=None):
    assert text not in ctx["page"]
