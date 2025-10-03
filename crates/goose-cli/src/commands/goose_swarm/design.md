Goose swarm design.

# High level design
Aim is for this command to find things to work on in github, claim and then process accordingly depending on what role they take on. Effectively swarming.
There will be many of these running disconnected from each other.

Roles are planner/orchestrator, and worker/drone at this time.
The swarm node will find someting to do, and pick which role needs to do it.
The swarm node continues in the mode it has picked up until done, and then goes back to polling for help wanted which can cause it to switch to any role depending on needs. In future may be other roles, but for now just 2. 

Most of the work happens in rust code, it runs as a big loop, and it can call the gh cli, and then the agent processing happens in goose recipes.
In some cases when in a role, it will poll in a loop for things to happen, but it is still in that inner loop. 

## starting up

The swarm node (really just the goose swarm command!) will startup, generate an id if needed (constant deterministic per machine). 

The node should post itself to an issue called "goose swarm" in the title, creating if it doesn't exist. If it does exist, then it should ensure its goose:node:node-123 id is visible in the body, one node entry per line:

looking for issue: gh issue list --repo michaelneale/test --search "#gooseswarm"
editing if existing: gh issue edit 1 --repo michaelneale/test --body $'EXISTING CONTENT HERE\ngoose:node:id_here'
creating if not: gh issue create --repo michaelneale/test --title "#gooseswarm" --body "goose:node:_ID_HERE"

## Finding something to do

list for issues that need help wanted, this can be run on a polling basis for the node to find some work to do:

gh issue list --repo michaelneale/test --label "help wanted" --state open --json number --jq '.[].number' | tr ',' ' '

### Identifying what role needed

How to know what kind of role: if it has a help wanted label, and the issue starts with [task] - then assume it is an unclaimed drone/worker task.
If it has a help wanted label and no [task] prefix in the title, then it is needing someone to take on the planner role for it. 

### Claiming the issue

Once a need has been identified, then the help wanted label should be removed and then update the issue body something like this: 

gh issue edit 1 --repo michaelneale/test --remove-label "help wanted"
gh issue edit 1 --repo michaelneale/test --body $'*add existing here*<details><summary>Goose planner</summary>\n<p>\ngoose:swarm:thing-0911-boop</details>'

In this example "planner" is the role. You should check there isn't already something in there claiming it (a double check). This mutates the body.


## Being the planner

Ok now you know you are the planner, you will load relevant context from the issue, and also load up the "goose swarm" titled issue, count how many goose:swarm items there are, and subtract 1 from it. 

run the planner.yaml goose recipe, with the issue context and repo name, and how many drone nodes are available (we will work on the content of that recipe later). The planner should run in a dir which has tasks dir, and issues dir, all empty to start. 

After recipe has run, look in tasks dir, and for each create a new issue with [task] and a name from the first line of each .md file you find, the rest is the body. label it "help wanted". Ensure body has "for:link-to-original-issue" in it.

eg: 
gh issue create --repo michaelneale/test --title "[task] title here" --body "rest of content from md here ... for:link-to-original-issue" --label "help wanted"

For each .md in issues, do the same but no [task] prefix to the name. 


Based on how many task issues (N) you created from tasks dir: wait for pull requests to come in addressing those N issues (can search for PRs in apporpriate state mentioning this parent issue), something like: 

gh pr list --repo michaelneale/test --search "for:link-to-original-issue in:body,title" --state all --json number,title,url

(you can work that exact commands out). This is done as a slow poll, as it may take some time for them to trickle in. You should wait for all PRs to be out of draft state (out of draft means ready - if draft, we ignore it)

We are looking for clean PRs, ie ones that have green builds and are open and available to merge. Can also look for closed ones (ie ones that were tried and given up) as they count. Once we have N of them, clean or closed, ie one for each issue, then we run the next recipe: 

evaluate.yaml

this recipe will take links to all these PRs, and content from the original issue. Its job will be to judge if each one should be merged and merge them, closed, perhaps pick a winner etc (details will be in that yaml I will do). Again, run in a clean dir but with issues subdir only this time.

once finished, close the main issue, and any [tasks] you spawned, 
Open new issues with any .md files found in the issues dir (first line being title)

This ends the tour of duty of being a planner.




## Being the drone

Ok you are the drone worker. that means you have claimed an issue with [task] prefix.

You will run swarm_drone.yaml recipe, with content from that task issue.
This runs in a dir with the cloned version of the repo, up to date with main (will need to ensure we clone it if not already).
Lets make it ~/.local/share/goose-swarm/repo-name-here

When this exits, check if the branch in that dir is still main - if it is, we assume failure, and we will add the help-wanted and remove the claim on the body of that task.

Otherwise, we are done.



#### Appendix with helpful gh cli commands

# list help wanted:
gh issue list --repo michaelneale/test --label "help wanted" --state open --json number --jq '.[].number' | tr ',' ' '

# remove help wanted label from issue id 1 when working on it
gh issue edit 1 --repo michaelneale/test --remove-label "help wanted"

# edit an issue body to mention your worker id (thing-0911-boop)
gh issue edit 1 --repo michaelneale/test --body $'*add existing here*<details><summary>Goose planner</summary>\n<p>\ngoose:swarm:thing-0911-boop</details>'

-------

# load all this issues details and comments into context
gh api graphql -f query='{ repository(owner:"michaelneale", name:"test") { issue(number:1) { body comments(first:100) { nodes { body } } } } }' --jq '.data.repository.issue | .body, .comments.nodes[].body'

# TIP for recipe: to push as a PR, linking back to the original issue id:
git checkout -b new-feature && git add . && git commit -m "Add new changes" && git push origin new-feature
gh pr create --title "Fix for issue #1" --body "Addresses #1 - and description from task issue here and also what has been done"


# load issue body and title into context: 
gh api graphql -f query='{ repository(owner:"michaelneale", name:"test") { issue(number:1) { body comments(first:100) { nodes { body } } } } }' --jq '.data.repository.issue | .body, .comments.nodes[].body'

# checking PR health
gh pr view 4557 --repo block/goose --json mergeable,mergeStateStatus,statusCheckRollup

