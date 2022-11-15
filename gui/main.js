class Status {
  #state = null;
  #tasksContainer = document.getElementById('tasks');
  #queueContainer = document.getElementById('queue');
  #libraryContainer = document.getElementById('library');

  display() {
    this.#tasksContainer.innerHTML = '';
    this.#queueContainer.innerHTML = '';
    this.#libraryContainer.innerHTML = '';
    this.#state.tasks.forEach((e) => {
      const item = document.createElement('div');
      item.className = 'media-item';
      item.innerHTML = `<div class="media-title">${e.title}</div><div>${e.link}</div>`;
      this.#tasksContainer.appendChild(item);
    });
    this.#state.queue.forEach((e) => {
      const item = document.createElement('div');
      item.className = 'media-item';
      item.innerHTML = `<div class="media-title">${e.title}</div><div>${e.link}</div>`;
      this.#queueContainer.appendChild(item);
    });
    this.#state.library.forEach((e) => {
      const item = document.createElement('div');
      item.className = 'media-item';
      item.innerHTML = `<div class="media-title">${e.title}</div>`;
      this.#libraryContainer.appendChild(item);
    });
  }

  load() {
    fetch('/state')
      .then((response) => response.json())
      .then((state) => {
        this.#state = state;
        this.display();
      });
  }
}

async function getFeed(url) {
  if (url.startsWith("www.youtube.com/c/") || url.startsWith("https://www.youtube.com/c/")) {
    return await fetch('/yt_feed', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        url: url
      }),
    }).then((response) => response.text());
  }
  return url;
}

class Sources {
  #sources = [];
  #container = document.getElementById('sources');
  #addButton = document.getElementById('source-add');
  #urlInput = document.getElementById('source-url');
  #typeInput = document.getElementById('source-type');

  constructor() {
    this.#addButton.addEventListener('click', () => this.addSource());
  }

  display() {
    this.#container.innerHTML = `<tr><th>URL</th><th>Type</th><th></th></tr>`;
    let i = 0;
    this.#sources.forEach((e) => {
      const row = document.createElement('tr');
      row.innerHTML = `<td>${e.url}</td><td>${e.type}</td>`;
      const removeButton = document.createElement('td');
      removeButton.className = 'remove-btn';
      removeButton.innerHTML = '✖';
      let curr = i;
      removeButton.addEventListener('click', () => this.removeSource(curr));
      row.appendChild(removeButton);
      this.#container.appendChild(row);
      i++;
    });
  }

  load() {
    fetch('/sources')
      .then((response) => response.json())
      .then((newSources) => {
        this.#sources = newSources;
        this.display();
      });
  }

  save() {
    fetch("/sources", {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(this.#sources),
    });
  }

  async addSource() {
    const url = this.#urlInput.value;
    const feedUrl = await getFeed(url);
    const type = this.#typeInput.value;
    this.#urlInput.value = '';
    const source = { url: feedUrl, type: type };
    this.#sources.push(source)
    this.display();
    this.save();
  }

  removeSource(index) {
    this.#sources.splice(index, 1);
    this.display();
    this.save();
  }
};

const DataType = Object.freeze({
  STRING: Symbol('string'),
  NUMBER: Symbol('number'),
});

const configMap = {
  sync_interval: DataType.NUMBER,
  parallel_downloads: DataType.NUMBER,
  media_dir: DataType.STRING,
  address: DataType.STRING,
  port: DataType.NUMBER,
  download_filter: {
    max_age: DataType.STRING,
    before: DataType.STRING,
    after: DataType.STRING,
  }
}

class Config {
  #config = null;
  #inputs = {};
  #saveButton = document.getElementById('config-save');

  constructor() {
    this.initInputs(configMap, this.#inputs, null);
    this.#saveButton.addEventListener('click', () => this.save());
  }

  initInputs(map, inputs, prefix) {
    for (const [key, type] of Object.entries(map)) {
      if (type === Object(type)) {
        inputs[key] = {};
        this.initInputs(type, inputs, key);
      } else {
        if (prefix == null) {
          inputs[key] = {
            element: document.getElementById(`config:${key}`),
            type: type,
          };
        } else {
          inputs[prefix][key] = {
            element: document.getElementById(`config:${prefix}.${key}`),
            type: type,
          };
        }
      }
    }
  }

  display() {
    this.displayConfig(this.#inputs, this.#config);
  }

  displayConfig(inputs, config, prefix) {
    for (const [key, value] of Object.entries(config)) {
      if (value === Object(value)) {
        this.displayConfig(inputs, value, key);
      } else {
        if (prefix == null) {
          inputs[key].element.value = value;
        } else {
          inputs[prefix][key].element.value = value;
        }
      }
    }
  }

  load() {
    fetch('/config')
      .then((response) => response.json())
      .then((config) => {
        this.#config = config;
        this.display();
      });
  }

  save() {
    this.#config = this.getConfig(this.#inputs);
    fetch("/config", {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(this.#config),
    });
  }

  getConfig(inputs) {
    let config = {};
    for (const [key, input] of Object.entries(inputs)) {
      let value;
      if (input.type == null) {
        value = this.getConfig(input);
      } else if (input.type == DataType.NUMBER) {
        value = input.element.valueAsNumber;
      } else if (input.type == DataType.STRING) {
        value = input.element.value;
      } else {
        console.error('Unkown data type!', input);
      }
      if (value != "") {
        config[key] = value;
      }
    }
    return config;
  }

}

const status = new Status();
const sources = new Sources();
const config = new Config();
status.load();
sources.load();
config.load();

