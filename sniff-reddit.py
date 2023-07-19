import datetime
import json

import praw

# https://praw.readthedocs.io/en/stable/getting_started/authentication.html
# Create a "personal use-script" app there https://www.reddit.com/prefs/apps
# (random redirect uri)
reddit = praw.Reddit(
    client_id="FILL ME", # In app page
    client_secret="FILL ME", # In app page
    password="FILL ME", # Reddit account password
    username="FILL ME", # Reddit username (not email)
    user_agent="testscript by u/fakebot3", # Unimportant
)
print(reddit.user.me()) # If this passes, you're logged in
subreddit = reddit.subreddit("hexcellslevels")
rows = []
for i, sub in enumerate(subreddit.top(limit=1000000)):
    dt = datetime.datetime.fromtimestamp(sub.created)
    t = dt.strftime("%Y-%m-%d")
    u = "[deleted]" if sub.author is None else sub.author.name
    print('{:4} {:3} {:10} {:30} {} {}'.format(i, sub.score, t, u, sub.title, sub.url))
    rows.append(dict(
        score=int(sub.score),
        title=sub.title,
        author=u,
        url=sub.url,
        date=t,
    ))
open('hexcellslevels.json', 'w').write(json.dumps(rows))
#
