const tasksContainer = document.getElementById('tasks');
const queueContainer = document.getElementById('queue');
const sourcesContainer = document.getElementById('sources');
const libraryContainer = document.getElementById('library');

const sourceAddButton = document.getElementById('source-add');
const sourceUrlInput = document.getElementById('source-url');
const sourceTypeInput = document.getElementById('source-type');

let sources = [];

function getSources() {
  fetch('/sources')
    .then((response) => response.json())
    .then((newSources) => {
      sources = newSources;
      displaySources();
    });
}

function setSources() {
  fetch("/sources", {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(sources),
  });
}

function displaySources() {
  sourcesContainer.innerHTML = `<tr><th>URL</th><th>Type</th><th></th></tr>`;
  let i = 0;
  sources.forEach((e) => {
    const row = document.createElement('tr');
    row.innerHTML = `<td>${e.url}</td><td>${e.type}</td>`;
    const removeButton = document.createElement('td');
    removeButton.className = 'remove-btn';
    removeButton.innerHTML = '✖';
    let curr = i;
    removeButton.addEventListener('click', () => removeSource(curr));
    row.appendChild(removeButton);
    sourcesContainer.appendChild(row);
    i++;
  });
}

function removeSource(index) {
  sources.splice(index, 1);
  setSources();
  displaySources();
}

async function getYoutubeFeed(url) {
  let feed = await fetch('/yt_feed', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      url: url
    }),
  }).then((response) => response.text())
  return feed;
}


async function getFeed(url) {
  if (url.startsWith("www.youtube.com/c/") || url.startsWith("https://www.youtube.com/c/")) {
    return await getYoutubeFeed(url);
  }
  return url;
}

async function addSource() {
  const url = sourceUrlInput.value;
  const feedUrl = await getFeed(url);
  const type = sourceTypeInput.value;
  sourceUrlInput.value = '';
  const source = { url: feedUrl, type: type };
  sources.push(source)
  setSources();
  displaySources();
}

function update() {
  fetch('/tasks')
    .then((response) => response.json())
    .then((tasks) => {
      tasksContainer.innerHTML = '';
      tasks.forEach((e) => {
        const item = document.createElement('li');
        item.innerHTML = `${e.title} (${e.link})`;
        tasksContainer.appendChild(item);
      });
    });

  fetch('/queue')
    .then((response) => response.json())
    .then((queue) => {
      queueContainer.innerHTML = '';
      queue.forEach((e) => {
        const item = document.createElement('li');
        item.innerHTML = `${e.title} (${e.link})`;
        queueContainer.appendChild(item);
      });
    });

  fetch('/library')
    .then((response) => response.json())
    .then((library) => {
      libraryContainer.innerHTML = '';
      library.forEach((e) => {
        const item = document.createElement('li');
        item.innerHTML = `${e.title}`;
        libraryContainer.appendChild(item);
      });
    });
}

sourceAddButton.addEventListener('click', () => addSource());
getSources();
update();

