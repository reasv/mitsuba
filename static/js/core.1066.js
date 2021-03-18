var $L = {
    nws: {"aco":1,"b":1,"bant":1,"d":1,"e":1,"f":1,"gif":1,"h":1,"hc":1,"hm":1,"hr":1,"i":1,"ic":1,"pol":1,"r":1,"r9k":1,"s":1,"s4s":1,"soc":1,"t":1,"trash":1,"u":1,"wg":1,"y":1},
    blue: '4channel.org', red: '4chan.org',
    d: function(b) {
      return $L.nws[b] ? $L.red : $L.blue;
    }
  };
  
  /**
   * Tooltips
   */
  var Tip = {
    node: null,
    timeout: null,
    delay: 300,
    
    init: function() {
      document.addEventListener('mouseover', this.onMouseOver, false);
      document.addEventListener('mouseout', this.onMouseOut, false);
    },
    
    onMouseOver: function(e) {
      var cb, data, t;
      
      t = e.target;
      
      if (Tip.timeout) {
        clearTimeout(Tip.timeout);
        Tip.timeout = null;
      }
      
      if (t.hasAttribute('data-tip')) {
        data = null;
        
        if (t.hasAttribute('data-tip-cb')) {
          cb = t.getAttribute('data-tip-cb');
          if (window[cb]) {
            data = window[cb](t);
          }
        }
        Tip.timeout = setTimeout(Tip.show, Tip.delay, e.target, data);
      }
    },
    
    onMouseOut: function(e) {
      if (Tip.timeout) {
        clearTimeout(Tip.timeout);
        Tip.timeout = null;
      }
      
      Tip.hide();
    },
    
    show: function(t, data, pos) {
      var el, rect, style, left, top;
      
      rect = t.getBoundingClientRect();
      
      el = document.createElement('div');
      el.id = 'tooltip';
      
      if (data) {
        el.innerHTML = data;
      }
      else {
        el.textContent = t.getAttribute('data-tip');
      }
      
      if (!pos) {
        pos = 'top';
      }
      
      el.className = 'tip-' + pos;
      
      document.body.appendChild(el);
      
      left = rect.left - (el.offsetWidth - t.offsetWidth) / 2;
      
      if (left < 0) {
        left = rect.left + 2;
        el.className += '-right';
      }
      else if (left + el.offsetWidth > document.documentElement.clientWidth) {
        left = rect.left - el.offsetWidth + t.offsetWidth + 2;
        el.className += '-left';
      }
      
      top = rect.top - el.offsetHeight - 5;
      
      style = el.style;
      style.top = (top + window.pageYOffset) + 'px';
      style.left = left + window.pageXOffset + 'px';
      
      Tip.node = el;
    },
    
    hide: function() {
      if (Tip.node) {
        document.body.removeChild(Tip.node);
        Tip.node = null;
      }
    }
  }
  
  /**
   * Settings Syncher
   */
  var StorageSync = {
    queue: [],
    
    init: function() {
      var el, self = StorageSync;
      
      if (self.inited || !document.body) {
        return;
      }
      
      self.remoteFrame = null;
      
      self.remoteOrigin = location.protocol + '//boards.'
        + (location.host === 'boards.4channel.org' ? '4chan' : '4channel')
        + '.org';
      
      window.addEventListener('message', self.onMessage, false);
      
      el = document.createElement('iframe');
      el.width = 0;
      el.height = 0;
      el.style.display = 'none';
      el.style.visibility = 'hidden';
      
      el.src = self.remoteOrigin + '/syncframe.html';
      
      document.body.appendChild(el);
      
      self.inited = true;
    },
    
    onMessage: function(e) {
      var self = StorageSync;
      
      if (e.origin !== self.remoteOrigin) {
        return;
      }
      
      if (e.data === 'ready') {
        self.remoteFrame = e.source;
        
        if (self.queue.length) {
          self.send();
        }
        
        return;
      }
    },
    
    sync: function(key) {
      var self = StorageSync;
      
      self.queue.push(key);
      self.send();
    },
    
    send: function() {
      var i, key, data, self = StorageSync;
      
      if (!self.inited) {
        return self.init();
      }
      
      if (!self.remoteFrame) {
        return;
      }
      
      data = {};
      
      for (i = 0; key = self.queue[i]; ++i) {
        data[key] = localStorage.getItem(key);
      }
      
      self.queue = [];
      
      self.remoteFrame.postMessage({ storage: data }, self.remoteOrigin);
    }
  };
  
  function mShowFull(t) {
    var el, data;
    
    if (t.className === 'name') {
      if (el = t.parentNode.parentNode.parentNode
          .getElementsByClassName('name')[1]) {
        data = el.innerHTML;
      }
    }
    else if (t.parentNode.className === 'subject') {
      if (el = t.parentNode.parentNode.parentNode.parentNode
          .getElementsByClassName('subject')[1]) {
        data = el.innerHTML;
      }
    }
    else if (/fileThumb/.test(t.parentNode.className)) {
      if (el = t.parentNode.parentNode.getElementsByClassName('fileText')[0]) {
        el = el.firstElementChild;
        data = el.getAttribute('title') || el.innerHTML;
      }
    }
    
    return data;
  }
  
  function loadBannerImage() {
    var cnt, el;
    
    cnt = document.getElementById('bannerCnt');
    
    if (!cnt || cnt.offsetWidth <= 0) {
      return;
    }
    
    cnt.innerHTML = '<img alt="4chan" src="/static/image/title/'
      + cnt.getAttribute('data-src') + '">';
  }
  
  function onMobileSelectChange() {
    var board, page;
    
    board = this.options[this.selectedIndex].value;
    page = (board !== 'f' && /\/catalog$/.test(location.pathname)) ? 'catalog' : '';
    
    window.location = '//boards.' + $L.d(board) + '/' + board + '/' + page;
  }
  
  function buildMobileNav() {
    var el, cnt, boards, i, b, html, order;
    
    if (el = document.getElementById('boardSelectMobile')) {
      html = '';
      order = [];
      
      boards = document.querySelectorAll('#boardNavDesktop .boardList a');
      
      for (i = 0; b = boards[i]; ++i) {
        order.push(b);
      }
      
      order.sort(function(a, b) {
        if (a.textContent < b.textContent) {
          return -1;
        }
        if (a.textContent > b.textContent) {
          return 1;
        }
        return 0;
      });
      
      for (i = 0; b = order[i]; ++i) {
        html += '<option class="'
          + (b.parentNode.classList.contains('nwsb') ? 'nwsb' : '') + '" value="'
          + b.textContent + '">/'
          + b.textContent + '/ - '
          + b.title + '</option>';
      }
      
      el.innerHTML = html;
    }
  }
  
  function cloneTopNav() {
    var navT, navB, ref, el;
    
    navT = document.getElementById('boardNavDesktop');
    
    if (!navT) {
      return;
    }
    
    ref = document.getElementById('absbot');
    
    navB = navT.cloneNode(true);
    navB.id = navB.id + 'Foot';
    
    if (el = navB.querySelector('#navtopright')) {
      el.id = 'navbotright';
    }
    
    if (el = navB.querySelector('#settingsWindowLink')) {
      el.id = el.id + 'Bot';
    }
    
    document.body.insertBefore(navB, ref);
  }
  
  function initPass() {
    if (get_cookie("pass_enabled") == '1' || get_cookie('extra_path')) {
      window.passEnabled = true;
    }
    else {
      window.passEnabled = false;
    }
  }
  
  function initBlotter() {
    var mTime, seenTime, el;
    
    el = document.getElementById('toggleBlotter');
    
    if (!el) {
      return;
    }
    
    el.addEventListener('click', toggleBlotter, false);
    
    seenTime = localStorage.getItem('4chan-blotter');
    
    if (!seenTime) {
      return;
    }
    
    mTime = +el.getAttribute('data-utc');
    
    if (mTime <= +seenTime) {
      toggleBlotter();
    }
  }
  
  function toggleBlotter(e) {
    var el, btn;
    
    e && e.preventDefault();
    
    el = document.getElementById('blotter-msgs');
    
    if (!el) {
      return;
    }
    
    btn = document.getElementById('toggleBlotter');
    
    if (el.style.display == 'none') {
      el.style.display = '';
      localStorage.removeItem('4chan-blotter');
      btn.textContent = 'Hide';
      
      el = btn.nextElementSibling;
      
      if (el.style.display) {
        el.style.display = '';
      }
    }
    else {
      el.style.display = 'none';
      localStorage.setItem('4chan-blotter', btn.getAttribute('data-utc'));
      btn.textContent = 'Show Blotter';
      btn.nextElementSibling.style.display = 'none';
    }
  }
  
  function onRecaptchaLoaded() {
    if (document.getElementById('postForm').style.display == 'table') {
      initRecaptcha();
    }
  }
  
  function initRecaptcha() {
    var el;
    
    el = document.getElementById('g-recaptcha');
    
    if (!el || el.firstElementChild) {
      return;
    }
    
    if (!window.passEnabled && window.grecaptcha) {
      grecaptcha.render(el, {
        sitekey: window.recaptchaKey,
        theme: (activeStyleSheet === 'Tomorrow' || window.dark_captcha) ? 'dark' : 'light'
      });
    }
  }
  
  function initAnalytics() {
    var tid = location.host.indexOf('.4channel.org') !== -1 ? 'UA-166538-5' : 'UA-166538-1';
    
    (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){(i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o),m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m)})(window,document,'script','//www.google-analytics.com/analytics.js','ga');
    
    ga('create', tid, {'sampleRate': 1});
    ga('set', 'anonymizeIp', true);
    ga('send','pageview');
  }
  
  function initAds(category, board) {
    var p = "http", d = "s", el;
    
    if (document.location.protocol == "https:") {
      p += "s";
    }
    
    var z = document.createElement("script");
    z.type = "text/javascript";
    z.async = true;
    z.src = p + "://" + d + ".zkcdn.net/ados.js";
    z.onload = function() {
      ados = ados || {};
      ados.run = ados.run || [];
      ados.run.push(function () {
        if (!(el = document.getElementById('azk91603'))) {
          return;
        }
        if (window.matchMedia && window.matchMedia('(max-width: 480px)').matches && localStorage.getItem('4chan_never_show_mobile') != 'true') {
          el.id = 'azk92421';
          window._top_ad = ados_add_placement(3536, 18130, "azk92421", 5).setZone(162389);
        }
        else {
          window._top_ad = ados_add_placement(3536, 18130, "azk91603", 4).setZone(16258);
        }
        ados_setPassbackTimeout(3000);
        ados_setDomain('engine.4chan-ads.org');
        ados_setKeywords(category + ', ' + board + (window.thread_archived ? ',archive' : ''));
        ados_setNoTrack();
        ados_load();
      });
    };
    
    var s = document.getElementsByTagName("script")[0];
    s.parentNode.insertBefore(z, s);
  }
  
  function initAdsAT() {
    var i, s, el, nodes;
    
    if (window.matchMedia && window.matchMedia('(max-width: 480px)').matches) {
      return;
    }
    
    nodes = document.getElementsByClassName('adt-800');
    
    for (i = 0; el = nodes[i]; ++i) {
      s = document.createElement('script');
      s.src = '//' + el.getAttribute('data-d') + '/' + el.id.replace('container-', '') + '/invoke.js';
      document.body.appendChild(s);
    }
  }
  
  function initAdsBG() {
    var i, s, el, div, nodes, m, idx;
    
    nodes = document.getElementsByClassName('adc-resp-bg');
    
    if (window.matchMedia && window.matchMedia('(max-width: 480px)').matches) {
      idx = 4;
    }
    else {
      idx = 0;
    }
    
    for (i = 0; el = nodes[i]; ++i) {
      m = el.getAttribute('data-ad-bg').split(',').slice(idx);
      
      if (m[0] === '0') {
        el.style.display = 'none';
        continue;
      }
      
      div = document.createElement('div');
      div.id = 'bg_' + m[0] + m[1] + m[2];
      el.appendChild(div);
      
      s = document.createElement('script');
      s.setAttribute('async', '');
      s.src = '//platform.bidgear.com/async.php?domainid=' + m[0] + '&sizeid=' + m[1] + '&zoneid=' + m[2] + '&k=' + Date.now();
      document.body.appendChild(s);
    }
  }
  
  function initAdsLD() {
    var i, j, p, el, div, nodes, m, idx;
    
    window.ldAdInit = window.ldAdInit || [];
    
    nodes = document.getElementsByClassName('adc-ld');
    
    if (!nodes[0]) {
      return;
    }
    
    if (window.matchMedia && window.matchMedia('(max-width: 480px)').matches) {
      idx = 1;
    }
    else {
      idx = 0;
    }
    
    for (i = 0; el = nodes[i]; ++i) {
      m = el.getAttribute('data-ld').split(',').slice(idx);
      window.ldAdInit.push({ slot: m, size: [0, 0], id: el.getAttribute('data-ld-id') });
    }
    
    if (!document.getElementById('ld-ajs')) {
      j = document.createElement('script');
      p = document.getElementsByTagName('script')[0];
      j.async = true;
      j.src = '//cdn2.lockerdomecdn.com/_js/ajs.js';
      j.id = 'ld-ajs';
      p.parentNode.insertBefore(j, p);
    }
  }
  
  function initAdsBGLS() {
    var i, s, el, m;
    
    m = window.matchMedia && window.matchMedia('(max-width: 480px)').matches;
    
    nodes = document.getElementsByClassName('ad-bgls');
    
    for (i = 0; el = nodes[i]; ++i) {
      if (m) {
        if (!el.hasAttribute('data-m')) {
          continue;
        }
      }
      else if (el.hasAttribute('data-m')) {
        continue;
      }
      
      s = document.createElement('script');
      s.async = true;
      s.src = 'https://bid.glass/unit/' + el.getAttribute('data-u') + '.js';
      document.head.appendChild(s);
    }
  }
  
  function initAdsAG() {
    var el, nodes, i, cls, s;
    
    if (window.matchMedia && window.matchMedia('(max-width: 480px)').matches && localStorage.getItem('4chan_never_show_mobile') != 'true') {
      cls = 'adg-m';
    }
    else {
      cls = 'adg';
    }
    
    nodes = document.getElementsByClassName(cls);
    
    for (i = 0; el = nodes[i]; ++i) {
      if (el.hasAttribute('data-rc')) {
        s = document.createElement('script');
        s.text = '(function(){var referer="";try{if(referer=document.referrer,"undefined"==typeof referer||""==referer)throw"undefined";}catch(exception){referer=document.location.href,(""==referer||"undefined"==typeof referer)&&(referer=document.URL)}referer=referer.substr(0,700);var rcds=document.getElementById("' + el.id + '");var rcel=document.createElement("script");rcel.id="rc_"+Math.floor(Math.random()*1E3);rcel.type="text/javascript";rcel.src="//trends.revcontent.com/serve.js.php?w=' + el.getAttribute('data-rc') + '&t="+rcel.id+"&c="+(new Date).getTime()+"&width="+(window.outerWidth||document.documentElement.clientWidth)+"&referer="+encodeURIComponent(referer);rcel.async=true;rcds.appendChild(rcel)})();';
        document.body.appendChild(s);
      }
      else if (el.hasAttribute('data-ja')) {
        s = document.createElement('iframe');
        s.setAttribute('scrolling', 'no');
        s.setAttribute('border', '0');
        s.setAttribute('frameborder', '0');
        s.setAttribute('allowtransparency', 'true');
        s.setAttribute('marginheight', '0');
        s.setAttribute('marginwidth', '0');
        s.setAttribute('width', el.getAttribute('data-adw') || '728');
        s.setAttribute('height', el.getAttribute('data-adh') || '102');
        s.src = '//adserver.juicyads.com/adshow.php?adzone=' + el.getAttribute('data-ja');
        el.appendChild(s);
      }
      else if (el.hasAttribute('data-adn')) {
        s = document.createElement('script');
        s.text = '(function(parentNode){var adnOpt={"id":' + el.id.replace('adn-', '') + ',"pid":6099,"sid":1291218,"type":6,"width":300,"height":250};var adn=document.createElement("script");adn.type="text/javascript";adn.async=true;adn.src="//a.adnium.com/static?r="+Math.floor(Math.random()*99999999)+"&id="+adnOpt.id+"&pid="+adnOpt.pid+"&sid="+adnOpt.sid+"&tid="+adnOpt.type+"&w="+adnOpt.width+"&h="+adnOpt.height;parentNode.appendChild(adn)})(document.getElementsByTagName("script")[document.getElementsByTagName("script").length-1].parentNode);';
        document.body.appendChild(s);
      }
      else if (el.hasAttribute('data-abc')) {
        s = document.createElement('iframe');
        s.setAttribute('scrolling', 'no');
        s.setAttribute('frameborder', '0');
        s.setAttribute('allowtransparency', 'true');
        s.setAttribute('marginheight', '0');
        s.setAttribute('marginwidth', '0');
        
        if (cls === 'adg') {
          s.setAttribute('width', '728');
          s.setAttribute('height', '90');
        }
        else {
          s.setAttribute('width', '300');
          s.setAttribute('height', '250');
        }
        
        s.setAttribute('name', 'spot_id_' + el.getAttribute('data-abc'));
        s.src = 'https://a.adtng.com/get/' + el.getAttribute('data-abc') + '?time=' + Date.now();
        el.appendChild(s);
      }
      else {
        s = document.createElement('script');
        s.src = '//community4chan.engine.adglare.net/?' + el.id.replace('zone', '');
        document.body.appendChild(s);
      }
    }
  }
  
  function applySearch(e) {
    var str;
    
    e && e.preventDefault();
    
    str = document.getElementById('search-box').value;
    
    if (str !== '') {
      window.location.href = 'catalog#s=' + str;
    }
  }
  
  function onKeyDownSearch(e) {
    if (e.keyCode == 13) {
      applySearch();
    }
  }
  
  function onReportClick(e) {
    var i, input, nodes, board;
    
    nodes = document.getElementsByTagName('input');
    
    board = location.pathname.split(/\//)[1];
    
    for (i = 0; input = nodes[i]; ++i) {
      if (input.type == 'checkbox' && input.checked && input.value == 'delete') {
        return reppop('https://sys.' + $L.d(board) + '/' + board + '/imgboard.php?mode=report&no='
          + input.name.replace(/[a-z]+/, '')
        );
      }
    }
  }
  
  function onStyleSheetChange(e) {
    setActiveStyleSheet(this.value);
  }
  
  function onPageSwitch(e) {
    e.preventDefault();
    window.location = this.action;
  }
  
  function onMobileFormClick(e) {
    var index = location.pathname.split(/\//).length < 4;
    
    e.preventDefault();
    
    if (this.parentNode.id == 'mpostform') {
      toggleMobilePostForm(index);
    }
    else {
      toggleMobilePostForm(index, 1);
    }
  }
  
  function onMobileRefreshClick(e) {
    locationHashChanged(this);
  }
  
  function toggle(name) {
    var a = document.getElementById(name);
    a.style.display = ((a.style.display != 'block') ? 'block' : 'none');
  }
  
  function quote(text) {
    if (document.selection) {
      document.post.com.focus();
      var sel = document.selection.createRange();
      sel.text = ">>" + text + "\n";
    } else if (document.post.com.selectionStart || document.post.com.selectionStart == "0") {
      var startPos = document.post.com.selectionStart;
      var endPos = document.post.com.selectionEnd;
      document.post.com.value = document.post.com.value.substring(0, startPos) + ">>" + text + "\n" + document.post.com.value.substring(endPos, document.post.com.value.length);
    } else {
      document.post.com.value += ">>" + text + "\n";
    }
  }
  
  function repquote(rep) {
    if (document.post.com.value == "") {
      quote(rep);
    }
  }
  
  function reppop(url) {
    var height, altc;
    
    if (window.passEnabled || !window.grecaptcha) {
      height = 205;
    }
    else {
      height = 510;
    }
    
    window.open(url, Date.now(), 
      'toolbar=0,scrollbars=1,location=0,status=1,menubar=0,resizable=1,width=380,height=' + height
    );
    
    return false;
  }
  
  function recaptcha_load() {
    var d = document.getElementById("recaptcha_div");
    if (!d) return;
  
    Recaptcha.create("6Ldp2bsSAAAAAAJ5uyx_lx34lJeEpTLVkP5k04qc", "recaptcha_div",{theme: "clean"});
  }
  
  function onParsingDone(e) {
    var i, nodes, n, p, tid, offset, limit;
  
    tid = e.detail.threadId;
    offset = e.detail.offset;
    
    if (!offset) {
      return;
    }
  
    nodes = document.getElementById('t' + tid).getElementsByClassName('nameBlock');
    limit = e.detail.limit ? (e.detail.limit * 2) : nodes.length;
    for (i = offset * 2 + 1; i < limit; i+=2) {
      if (n = nodes[i].children[1]) {
        if (currentHighlighted
          && n.className.indexOf('id_' + currentHighlighted) != -1) {
          p = n.parentNode.parentNode.parentNode;
          p.className = 'highlight ' + p.className;
        }
        n.addEventListener('click', idClick, false)
      }
    }
  }
  
  function loadExtraScripts() {
    var el, path;
    
    path = readCookie('extra_path');
    
    if (!path || !/^[a-z0-9]+$/.test(path)) {
      return false;
    }
    
    if (window.FC) {
      el = document.createElement('script');
      el.type = 'text/javascript';
      el.src = 'https://s.4cdn.org/js/' + path + '.' + jsVersion + '.js';
      document.head.appendChild(el);
    }
    else {
      document.write('<script type="text/javascript" src="https://s.4cdn.org/js/' + path + '.' + jsVersion + '.js"></script>');
    }
    
    return true;
  }
  
  
  function toggleMobilePostForm(index, scrolltotop) {
    var elem = document.getElementById('mpostform').firstElementChild;
    var postForm = document.getElementById('postForm');
    
    if (elem.className.match('hidden')) {
      elem.className = elem.className.replace('hidden', 'shown');
      postForm.className = postForm.className.replace(' hideMobile', '');
      elem.innerHTML = 'Close Post Form';
      initRecaptcha();
    }
    else {
      elem.className = elem.className.replace('shown', 'hidden');
      postForm.className += ' hideMobile';
      elem.innerHTML = (index) ? 'Start New Thread' : 'Post Reply';
    }
    
    if (scrolltotop) {
      elem.scrollIntoView();
    }
  }
  
  function toggleGlobalMessage(e) {
    var elem, postForm;
    
    if (e) {
      e.preventDefault();
    }
    
    elem = document.getElementById('globalToggle');
    postForm = document.getElementById('globalMessage');
  
    if( elem.className.match('hidden') ) {
      elem.className = elem.className.replace('hidden', 'shown');
      postForm.className = postForm.className.replace(' hideMobile', '');
  
      elem.innerHTML = 'Close Announcement';
    } else {
      elem.className = elem.className.replace('shown', 'hidden');
      postForm.className += ' hideMobile';
  
      elem.innerHTML = 'View Announcement';
    }
  }
  
  function checkRecaptcha()
  {
    if( typeof RecaptchaState.timeout != 'undefined' ) {
      if( RecaptchaState.timeout == 1800 ) {
        RecaptchaState.timeout = 570;
        Recaptcha._reset_timer();
        clearInterval(captchainterval);
      }
    }
  }
  
  function setPassMsg() {
    var el, msg;
    
    el = document.getElementById('captchaFormPart');
    
    if (!el) {
      return;
    }
    
    msg = 'You are using a 4chan Pass. [<a href="https://sys.' + $L.d(location.pathname.split(/\//)[1]) + '/auth?act=logout" onclick="confirmPassLogout(event);" tabindex="-1">Logout</a>]';
    el.children[1].innerHTML = '<div style="padding: 5px;">' + msg + '</div>';
  }
  
  function confirmPassLogout(event)
  {
    var conf = confirm('Are you sure you want to logout?');
    if( !conf ) {
      event.preventDefault();
      return false;
    }
  }
  
  var activeStyleSheet;
  
  function initStyleSheet() {
    var i, rem, link, len;
    
    // fix me
    if (window.FC) {
      return;
    }
    
    if (window.style_group) {
      var cookie = readCookie(style_group);
      activeStyleSheet = cookie ? cookie : getPreferredStyleSheet();
    }
    
    if (window.css_event && localStorage.getItem('4chan_stop_css_event') !== window.css_event) {
      activeStyleSheet = '_special';
    }
    
    switch(activeStyleSheet) {
      case "Yotsuba B":
        setActiveStyleSheet("Yotsuba B New", true);
        break;
  
      case "Yotsuba":
        setActiveStyleSheet("Yotsuba New", true);
        break;
  
      case "Burichan":
        setActiveStyleSheet("Burichan New", true);
        break;
  
      case "Futaba":
        setActiveStyleSheet("Futaba New", true);
        break;
  
      default:
        setActiveStyleSheet(activeStyleSheet, true);
      break;
    }
    
    if (localStorage.getItem('4chan_never_show_mobile') == 'true') {
      link = document.querySelectorAll('link');
      len = link.length;
      for (i = 0; i < len; i++) {
        if (link[i].getAttribute('href').match('mobile')) {
          (rem = link[i]).parentNode.removeChild(rem);
        }
      }
    }
  }
  
  function pageHasMath() {
    var i, el, nodes;
    
    nodes = document.getElementsByClassName('postMessage');
    
    for (i = 0; el = nodes[i]; ++i) {
      if (/\[(?:eqn|math)\]|"math">/.test(el.innerHTML)) {
        return true;
      }
    }
    
    return false;
  }
  
  function cleanWbr(el) {
    var i, nodes, n;
    
    nodes = el.getElementsByTagName('wbr');
    
    for (i = nodes.length - 1; n = nodes[i]; i--) {
      n.parentNode.removeChild(n);
    }
  }
  
  function parseMath() {
    var i, el, nodes;
    
    nodes = document.getElementsByClassName('postMessage');
    
    for (i = 0; el = nodes[i]; ++i) {
      if (/\[(?:eqn|math)\]/.test(el.innerHTML)) {
        cleanWbr(el);
      }
    }
    
    MathJax.Hub.Queue(['Typeset', MathJax.Hub, nodes]);
  }
  
  function loadMathJax() {
    var head, script;
    
    head = document.getElementsByTagName('head')[0];
    
    script = document.createElement('script');
    script.type = 'text/x-mathjax-config';
    script.text = "MathJax.Hub.Config({\
  extensions: ['Safe.js'],\
  tex2jax: { processRefs: false, processEnvironments: false, preview: 'none', inlineMath: [['[math]','[/math]']], displayMath: [['[eqn]','[/eqn]']] },\
  Safe: { allow: { URLs: 'none', classes: 'none', cssIDs: 'none', styles: 'none', fontsize: 'none', require: 'none' } },\
  displayAlign: 'left', messageStyle: 'none', skipStartupTypeset: true,\
  'CHTML-preview': { disabled: true }, MathMenu: { showRenderer: false, showLocale: false },\
  TeX: { Macros: { color: '{}', newcommand: '{}', renewcommand: '{}', newenvironment: '{}', renewenvironment: '{}', def: '{}', let: '{}'}}});";
    head.appendChild(script);  
    
    script = document.createElement('script');
    script.src = '//cdn.mathjax.org/mathjax/2.6-latest/MathJax.js?config=TeX-AMS_HTML-full';
    script.onload = parseMath;
    head.appendChild(script);
  }
  
  captchainterval = null;
  function init() {
    var el;
    var error = typeof is_error != "undefined";
    var board = mitsuba_board;
    // var board = location.href.match(/(?:4chan|4channel)\.org\/(\w+)/)[1];
    var arr = location.href.split(/#/);
    if( arr[1] && arr[1].match(/q[0-9]+$/) ) {
      repquote( arr[1].match(/q([0-9]+)$/)[1] );
    }
  
  
    if (window.math_tags && pageHasMath()) {
      loadMathJax();
    }
  
    if(navigator.userAgent) {
      if( navigator.userAgent.match( /iP(hone|ad|od)/i ) ) {
        links = document.querySelectorAll('s');
        len = links.length;
  
        for( var i = 0; i < len; i++ ) {
          links[i].onclick = function() {
            if (this.hasAttribute('style')) {
              this.removeAttribute('style');
            }
            else {
              this.setAttribute('style', 'color: #fff!important;');
            }
          }
        }
      }
    }
  
    if( document.getElementById('styleSelector') ) {
          styleSelect = document.getElementById('styleSelector');
          len = styleSelect.options.length;
          for ( var i = 0; i < len; i++) {
              if (styleSelect.options[i].value == activeStyleSheet) {
                  styleSelect.selectedIndex = i;
                  continue;
              }
          }
      }
  
    if (!error && document.forms.post) {
      if (board != 'i' && board != 'ic' && board != 'f') {
        if (window.File && window.FileReader && window.FileList && window.Blob) {
          el = document.getElementById('postFile');
          el && el.addEventListener('change', handleFileSelect, false);
        }
      }
    }
  
    //window.addEventListener('onhashchange', locationHashChanged, false);
  
    if( typeof extra != "undefined" && extra && !error ) extra.init();
  }
  
  var coreLenCheckTimeout = null;
  function onComKeyDown() {
    clearTimeout(coreLenCheckTimeout);
    coreLenCheckTimeout = setTimeout(coreCheckComLength, 500);
  }
  
  function coreCheckComLength() {
    var byteLength, comField, error;
    
    if (comlen) {
      comField = document.getElementsByName('com')[0];
      byteLength = encodeURIComponent(comField.value).split(/%..|./).length - 1;
      
      if (byteLength > comlen) {
        if (!(error = document.getElementById('comlenError'))) {
          error = document.createElement('div');
          error.id = 'comlenError';
          error.style.cssText = 'font-weight:bold;padding:5px;color:red;';
          comField.parentNode.appendChild(error);
        }
        error.textContent = 'Error: Comment too long (' + byteLength + '/' + comlen + ').';
      }
      else if (error = document.getElementById('comlenError')) {
        error.parentNode.removeChild(error);
      }
    }
  }
  
  function disableMobile() {
    localStorage.setItem('4chan_never_show_mobile', 'true');
    location.reload(true);
  }
  
  function enableMobile() {
    localStorage.removeItem('4chan_never_show_mobile');
    location.reload(true);
  }
  
  var currentHighlighted = null;
  function enableClickableIds()
  {
    var i = 0, len = 0;
    var elems = document.getElementsByClassName('posteruid');
    var capcode = document.getElementsByClassName('capcode');
  
    if( capcode != null ) {
      for( i = 0, len = capcode.length; i < len; i++ ) {
        capcode[i].addEventListener("click", idClick, false);
      }
    }
  
    if( elems == null ) return;
    for( i = 0, len = elems.length; i < len; i++ ) {
      elems[i].addEventListener("click", idClick, false);
    }
  }
  
  function idClick(evt)
  {
    var i = 0, len = 0, node;
    var uid = evt.target.className == 'hand' ? evt.target.parentNode.className.match(/id_([^ $]+)/)[1] : evt.target.className.match(/id_([^ $]+)/)[1];
  
    // remove all .highlight classes
    var hl = document.getElementsByClassName('highlight');
    len = hl.length;
    for( i = 0; i < len; i++ ) {
      var cn = hl[0].className.toString();
      hl[0].className = cn.replace(/highlight /g, '');
    }
  
    if( currentHighlighted == uid ) {
      currentHighlighted = null;
      return;
    }
    currentHighlighted = uid;
  
    var nhl = document.getElementsByClassName('id_' + uid);
    len = nhl.length;
    for( i = 0; i < len; i++ ) {
      node = nhl[i].parentNode.parentNode.parentNode;
      if( !node.className.match(/highlight /) ) node.className = "highlight " + node.className;
    }
  }
  
  function showPostFormError(msg) {
    var el = document.getElementById('postFormError');
    
    if (msg) {
      el.innerHTML = msg;
      el.style.display = 'block';
    }
    else {
      el.textContent = '';
      el.style.display = '';
    }
  }
  
  function handleFileSelect() {
    var fsize, maxFilesize;
    
    if (this.files) {
      maxFilesize = window.maxFilesize;
      
      fsize = this.files[0].size;
      
      if (this.files[0].type == 'video/webm' && window.maxWebmFilesize) {
        maxFilesize = window.maxWebmFilesize;
      }
      
      if (fsize > maxFilesize) {
        showPostFormError('Error: Maximum file size allowed is '
          + Math.floor(maxFilesize / 1048576) + ' MB');
      }
      else {
        showPostFormError();
      }
    }
  }
  
  function locationHashChanged(e)
  {
    var css = document.getElementById('id_css');
  
    switch( e.id )
    {
      case 'refresh_top':
        url = window.location.href.replace(/#.+/, '#top');
        if( !/top$/.test(url) ) url += '#top';
        css.innerHTML = '<meta http-equiv="refresh" content="0;URL=' + url + '">';
        document.location.reload(true);
        break;
  
      case 'refresh_bottom':
        url = window.location.href.replace(/#.+/, '#bottom');
        if( !/bottom$/.test(url) ) url += '#bottom';
        css.innerHTML = '<meta http-equiv="refresh" content="0;URL=' + url + '">';
        document.location.reload(true);
        break;
  
      default:break;
    }
  
    return true;
  
  }
  
  function setActiveStyleSheet(title, init) {
    var a, link, href, i, nodes, fn;
    
    if( document.querySelectorAll('link[title]').length == 1 ) {
      return;
    }
    
    href = '';
    
    nodes = document.getElementsByTagName('link');
    
    for (i = 0; a = nodes[i]; i++) {
      if (a.getAttribute("title") == "switch") {
        link = a;
      }
      
      if (a.getAttribute("rel").indexOf("style") != -1 && a.getAttribute("title")) {
        if (a.getAttribute("title") == title) {
          href = a.href;
        }
      }
    }
  
    link && link.setAttribute("href", href);
  
    if (!init) {
      if (title !== '_special') {
        createCookie(style_group, title, 365, $L.d(location.pathname.split(/\//)[1]));
        
        if (window.css_event) {
          fn = window['fc_' + window.css_event + '_cleanup'];
          localStorage.setItem('4chan_stop_css_event', window.css_event);
        }
      }
      else if (window.css_event) {
        fn = window['fc_' + window.css_event + '_init'];
        localStorage.removeItem('4chan_stop_css_event');
      }
      
      StorageSync.sync('4chan_stop_css_event');
      
      activeStyleSheet = title;
      
      fn && fn();
    }
  }
  
  function getActiveStyleSheet() {
    var i, a;
    var link;
  
      if( document.querySelectorAll('link[title]').length == 1 ) {
          return 'Yotsuba P';
      }
  
    for (i = 0; (a = document.getElementsByTagName("link")[i]); i++) {
      if (a.getAttribute("title") == "switch")
                 link = a;
      else if (a.getAttribute("rel").indexOf("style") != -1 && a.getAttribute("title") && a.href==link.href) return a.getAttribute("title");
    }
    return null;
  }
  
  function getPreferredStyleSheet() {
    return (style_group == "ws_style") ? "Yotsuba B New" : "Yotsuba New";
  }
  
  function createCookie(name, value, days, domain) {
    if (days) {
      var date = new Date();
      date.setTime(date.getTime() + (days * 24 * 60 * 60 * 1000));
      var expires = "; expires=" + date.toGMTString();
    } else expires = "";
    if (domain) domain = "; domain=" + domain;
    else domain = "";
    document.cookie = name + "=" + value + expires + "; path=/" + domain;
  }
  
  function readCookie(name) {
    var nameEQ = name + "=";
    var ca = document.cookie.split(';');
    for (var i = 0; i < ca.length; i++) {
      var c = ca[i];
      while (c.charAt(0) == ' ') c = c.substring(1, c.length);
      if (c.indexOf(nameEQ) == 0) {
        return decodeURIComponent(c.substring(nameEQ.length, c.length));
      }
    }
    return '';
  }
  
  // legacy
  var get_cookie = readCookie;
  
  function setRetinaIcons() {
    var i, j, nodes;
    
    nodes = document.getElementsByClassName('retina');
    
    for (i = 0; j = nodes[i]; ++i) {
      j.src = j.src.replace(/\.(gif|png)$/, "@2x.$1");
    }
  }
  
  function onCoreClick(e) {
    if (/flag flag-/.test(e.target.className) && e.which == 1) {
      window.open('//s.4cdn.org/image/country/'
        + e.target.className.match(/flag-([a-z]+)/)[1]
        + '.gif', '');
    }
  }
  
  function showPostForm(e) {
    var el;
    
    e && e.preventDefault();
    
    if (el = document.getElementById('postForm')) {
      $.id('togglePostFormLink').style.display = 'none';
      el.style.display = 'table';
      initRecaptcha();
    }
  }
  
  function oeCanvasPreview(e) {
    var t, el, sel;
    
    if (el = document.getElementById('oe-canvas-preview')) {
      el.parentNode.removeChild(el);
    }
    
    if (e.target.nodeName == 'OPTION' && e.target.value != '0') {
      t = document.getElementById('f' + e.target.value);
      
      if (!t) {
        return;
      }
      
      t = t.getElementsByTagName('img')[0];
      
      if (!t || !t.hasAttribute('data-md5')) {
        return;
      }
      
      el = t.cloneNode();
      el.id = 'oe-canvas-preview';
      sel = e.target.parentNode;
      sel.parentNode.insertBefore(el, sel.nextSibling);
    }
  }
  
  function oeClearPreview(e) {
    var el;
    
    if (el = document.getElementById('oe-canvas-preview')) {
      el.parentNode.removeChild(el);
    }
  }
  
  var PainterCore = {
    init: function() {
      var cnt, btns;
      
      if (!document.forms.post) {
        return;
      }
      
      cnt = document.forms.post.getElementsByClassName('painter-ctrl')[0];
      
      if (!cnt) {
        return;
      }
      
      btns = cnt.getElementsByTagName('button');
      
      if (!btns[1]) {
        return;
      }
      
      this.data = null;
      this.replayBlob = null;
      
      this.time = 0;
      
      this.btnDraw = btns[0];
      this.btnClear = btns[1];
      this.btnFile = document.getElementById('postFile');
      this.btnSubmit = document.forms.post.querySelector('input[type="submit"]');
      this.inputNodes = cnt.getElementsByTagName('input');
      
      btns[0].addEventListener('click', this.onDrawClick, false);
      btns[1].addEventListener('click', this.onCancel, false);
    },
    
    onDrawClick: function() {
      var w, h, dims = this.parentNode.getElementsByTagName('input');
      
      w = +dims[0].value;
      h = +dims[1].value;
      
      if (w < 1 || h < 1) {
        return;
      }
      
      window.Keybinds && (Keybinds.enabled = false);
      
      Tegaki.open({
        onDone: PainterCore.onDone,
        onCancel: PainterCore.onCancel,
        saveReplay: PainterCore.replayCb && PainterCore.replayCb.checked,
        width: w,
        height: h
      });
    },
    
    replay: function(id) {
      id = +id;
      
      Tegaki.open({
        replayMode: true,
        replayURL: '//i.4cdn.org/' + location.pathname.split(/\//)[1] + '/' + id + '.tgkr'
      });
    },
    
    // move this to tegaki.js
    b64toBlob: function(data) {
      var i, bytes, ary, bary, len;
      
      bytes = atob(data);
      len = bytes.length;
      
      ary = new Array(len);
      
      for (i = 0; i < len; ++i) {
        ary[i] = bytes.charCodeAt(i);
      }
      
      bary = new Uint8Array(ary);
      
      return new Blob([bary]);
    },
    
    onDone: function() {
      var self, blob, el;
      
      self = PainterCore;
      
      window.Keybinds && (Keybinds.enabled = true);
      
      self.btnFile.disabled = true;
      self.btnClear.disabled = false;
      
      self.data = Tegaki.flatten().toDataURL('image/png');
      
      if (Tegaki.saveReplay) {
        self.replayBlob = Tegaki.replayRecorder.toBlob();
      }
      
      if (!Tegaki.hasCustomCanvas && Tegaki.startTimeStamp) {
        self.time = Math.round((Date.now() - Tegaki.startTimeStamp) / 1000);
      }
      else {
        self.time = 0;
      }
      
      self.btnFile.style.visibility = 'hidden';
      
      self.btnDraw.textContent = 'Edit';
      
      for (el of self.inputNodes) {
        el.disabled = true;
      }
      
      document.forms.post.addEventListener('submit', self.onSubmit, false);
    },
    
    onCancel: function() {
      var self = PainterCore;
      
      window.Keybinds && (Keybinds.enabled = true);
      
      self.data = null;
      self.replayBlob = null;
      self.time = 0;
      
      self.btnFile.disabled = false;
      self.btnClear.disabled = true;
      
      self.btnFile.style.visibility = '';
      
      self.btnDraw.textContent = 'Draw';
      
      for (el of self.inputNodes) {
        el.disabled = false;
      }
      
      document.forms.post.removeEventListener('submit', self.onSubmit, false);
    },
    
    onSubmit: function(e) {
      var formdata, blob, xhr;
      
      e.preventDefault();
      
      formdata = new FormData(this);
      
      blob = PainterCore.b64toBlob(PainterCore.data.slice(PainterCore.data.indexOf(',') + 1));
      
      if (blob) {
        formdata.append('upfile', blob, 'tegaki.png');
        
        if (PainterCore.replayBlob) {
          formdata.append('oe_replay', PainterCore.replayBlob, 'tegaki.tgkr');
        }
      }
      
      formdata.append('oe_time', PainterCore.time);
      
      xhr = new XMLHttpRequest();
      xhr.open('POST', this.action, true);
      xhr.withCredentials = true;
      xhr.onerror = PainterCore.onSubmitError;
      xhr.onload = PainterCore.onSubmitDone;
      
      xhr.send(formdata);
      
      PainterCore.btnSubmit.disabled = true;
    },
    
    onSubmitError: function() {
      PainterCore.btnSubmit.disabled = false;
      showPostFormError('Connection Error.');
    },
    
    onSubmitDone: function() {
      var resp, ids, tid, pid, board;
      
      PainterCore.btnSubmit.disabled = false;
      
      if (ids = this.responseText.match(/<!-- thread:([0-9]+),no:([0-9]+) -->/)) {
        tid = +ids[1];
        pid = +ids[2];
        
        if (!tid) {
          tid = pid;
        }
        
        board = location.pathname.split(/\//)[1];
        
        window.location.href = '/' + board + '/thread/' + tid + '#p' + pid;
        
        PainterCore.onCancel();
        
        if (tid != pid) {
          PainterCore.btnClear.disabled = true;
          window.location.reload();
        }
        
        return;
      }
      
      if (resp = this.responseText.match(/"errmsg"[^>]*>(.*?)<\/span/)) {
        showPostFormError(resp[1]);
      }
    }
  };
  
  function oeReplay(id) {
    PainterCore.replay(id);
  }
  
  function contentLoaded() {
    var i, el, el2, nodes, len, mobileSelect, params, board, val, fn;
    
    document.removeEventListener('DOMContentLoaded', contentLoaded, true);
    
    // initAdsAG();
    
    // initAdsAT();
    
    // initAdsBG();
    
    // initAdsLD();
    
    // initAdsBGLS();
    
    if (document.post) {
      document.post.name.value = get_cookie("4chan_name");
      document.post.email.value = get_cookie("options");
    }
    
    cloneTopNav();
    
    // initAnalytics();
    
    params = location.pathname.split(/\//);
    
    board = params[1];
    
    if (window.passEnabled) {
      setPassMsg();
    }
    
    if (window.Tegaki) {
      PainterCore.init();
    }
    
    if (el = document.getElementById('bottomReportBtn')) {
      el.addEventListener('click', onReportClick, false);
    }
    
    if (el = document.getElementById('styleSelector')) {
      el.addEventListener('change', onStyleSheetChange, false);
    }
    
    // Post form toggle
    if (el = document.getElementById('togglePostFormLink')) {
      if (el = el.firstElementChild) {
        el.addEventListener('click', showPostForm, false);
      }
      if (location.hash === '#reply') {
        showPostForm();
      }
    }
    
    // Selectable flags
    if ((el = document.forms.post) && el.flag) {
      if ((val = readCookie('4chan_flag')) && (el2 = el.querySelector('option[value="' + val + '"]'))) {
        el2.setAttribute('selected', 'selected');
      }
    }
    
    // Mobile nav menu
    buildMobileNav();
    
    // Mobile global message toggle
    if (el = document.getElementById('globalToggle')) {
      el.addEventListener('click', toggleGlobalMessage, false);
    }
    
    if (localStorage.getItem('4chan_never_show_mobile') == 'true') {
      if (el = document.getElementById('disable-mobile')) {
        el.style.display = 'none';
        el = document.getElementById('enable-mobile');
        el.parentNode.style.cssText = 'display: inline !important;';
      }
    }
    
    if (mobileSelect = document.getElementById('boardSelectMobile')) {
      len = mobileSelect.options.length;
      for ( i = 0; i < len; i++) {
        if (mobileSelect.options[i].value == board) {
          mobileSelect.selectedIndex = i;
          continue;
        }
      }
      
      mobileSelect.addEventListener('change', onMobileSelectChange, false);
    }
    
    if (document.forms.oeform && (el = document.forms.oeform.oe_src)) {
      el.addEventListener('mouseover', oeCanvasPreview, false);
      el.addEventListener('mouseout', oeClearPreview, false);
    }
    
    if (params[2] != 'catalog') {
      // Mobile post form toggle
      nodes = document.getElementsByClassName('mobilePostFormToggle');
      
      for (i = 0; el = nodes[i]; ++i) {
        el.addEventListener('click', onMobileFormClick, false);
      }
      
      if (el = document.getElementsByName('com')[0]) {
        el.addEventListener('keydown', onComKeyDown, false);
        el.addEventListener('paste', onComKeyDown, false);
        el.addEventListener('cut', onComKeyDown, false);
      }
      
      // Mobile refresh buttons
      if (el = document.getElementById('refresh_top')) {
        el.addEventListener('mouseup', onMobileRefreshClick, false);
      }
      
      if (el = document.getElementById('refresh_bottom')) {
        el.addEventListener('mouseup', onMobileRefreshClick, false);
      }
      
      // Clickable flags
      if (board == 'int' || board == 'sp' || board == 'pol') {
        el = document.getElementById('delform');
        el.addEventListener('click', onCoreClick, false);
      }
      
      // Page switcher + Search field
      if (!params[3]) {
        nodes = document.getElementsByClassName('pageSwitcherForm');
        
        for (i = 0; el = nodes[i]; ++i) {
          el.addEventListener('submit', onPageSwitch, false);
        }
        
        if (el = document.getElementById('search-box')) {
          el.addEventListener('keydown', onKeyDownSearch, false);
        }
      }
      
      if (window.clickable_ids) {
        enableClickableIds();
      }
      
      Tip.init();
    }
    
    if (window.devicePixelRatio >= 2) {
      setRetinaIcons();
    }
    
    initBlotter();
    
    loadBannerImage();
    
    if (window.css_event && activeStyleSheet === '_special') {
      fn = window['fc_' + window.css_event + '_init'];
      fn && fn();
    }
  }
  
  initPass();
  
  window.onload = init;
  
  if (window.clickable_ids) {
    document.addEventListener('4chanParsingDone', onParsingDone, false);
  }
  
  document.addEventListener('4chanMainInit', loadExtraScripts, false);
  document.addEventListener('DOMContentLoaded', contentLoaded, true);
  
  initStyleSheet();